//! Filesystem, git, and misc helpers.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Read a file to string, returning None if it does not exist / cannot be read.
pub fn read_opt(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Canonical bytes representing a clean or uninitialized gitlink pointing at `sha`.
/// Shared by `read_scope_bytes`, `speclink-host::change_diff`, and desktop remote freshness
/// so every consumer produces identical bytes.
pub fn gitlink_canonical_bytes(sha: &str) -> Vec<u8> {
    format!("Subproject commit {sha}\n").into_bytes()
}

/// Normalize a repo-relative path for git pathspec/output comparison:
/// forward slashes, collapses `.` and repeated `/`, no trailing slash. Rejects empty paths,
/// NUL bytes, absolute paths, drive letters, and `..` traversal so callers cannot escape `root`.
fn normalize_git_rel_path(rel_path: &str) -> Option<String> {
    if rel_path.contains('\0') {
        return None;
    }
    let replaced = rel_path.replace('\\', "/");
    if replaced.starts_with('/') {
        return None;
    }
    let mut parts: Vec<&str> = Vec::new();
    for seg in replaced.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." || seg.contains(':') {
            return None;
        }
        parts.push(seg);
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts.join("/"))
}

fn is_git_hex_oid(s: &str) -> bool {
    s.len() >= 40 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// True when `sub_dir` is itself the top-level work tree of its own git checkout
/// (never climbing into the parent repository when `sub_dir/.git` is an empty/broken
/// directory or a dangling symlink).
fn is_own_git_worktree(sub_dir: &Path) -> bool {
    let mut cmd = git_command(sub_dir);
    if let Some(parent) = sub_dir.parent() {
        cmd.env("GIT_CEILING_DIRECTORIES", parent);
    }
    let Ok(out) = cmd.args(["rev-parse", "--show-toplevel"]).output() else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    let top = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if top.is_empty() {
        return false;
    }
    match (std::fs::canonicalize(&top), std::fs::canonicalize(sub_dir)) {
        (Ok(a), Ok(b)) => a == b,
        _ => Path::new(&top) == sub_dir,
    }
}

/// Run `git -C <root> <args...>`, returning raw stdout bytes on success.
fn git_bytes_opt(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = git_command(root).args(args).output().ok()?;
    output.status.success().then_some(output.stdout)
}

/// Inspect `<rev>` tree in `root` for a tree mode `160000` gitlink at `rel_path`,
/// returning its recorded commit SHA. Returns `None` for regular files, plain
/// directories, missing paths, or non-git checkouts.
pub fn gitlink_commit_at(root: &Path, rev: &str, rel_path: &str) -> Option<String> {
    let norm = normalize_git_rel_path(rel_path)?;
    if rev.is_empty() || rev.starts_with('-') {
        return None;
    }
    let raw = git_bytes_opt(
        root,
        &["-c", "core.quotepath=false", "ls-tree", "-z", rev, "--", &norm],
    )?;
    for entry in raw.split(|&b| b == 0).filter(|e| !e.is_empty()) {
        let line = String::from_utf8_lossy(entry);
        let Some((meta, path)) = line.split_once('\t') else {
            continue;
        };
        if path != norm {
            continue;
        }
        let mut parts = meta.split_whitespace();
        let (Some(mode), Some(kind), Some(sha)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        if mode == "160000" && kind == "commit" && is_git_hex_oid(sha) {
            return Some(sha.to_string());
        }
    }
    None
}

/// Inspect the parent index in `root` for a mode `160000` gitlink entry at `rel_path`,
/// returning its staged commit SHA (preferring stage 0).
fn gitlink_index_commit(root: &Path, rel_path: &str) -> Option<String> {
    let norm = normalize_git_rel_path(rel_path)?;
    let raw = git_bytes_opt(
        root,
        &["-c", "core.quotepath=false", "ls-files", "--stage", "-z", "--", &norm],
    )?;
    let mut fallback: Option<String> = None;
    for entry in raw.split(|&b| b == 0).filter(|e| !e.is_empty()) {
        let line = String::from_utf8_lossy(entry);
        let Some((meta, path)) = line.split_once('\t') else {
            continue;
        };
        if path != norm {
            continue;
        }
        let mut parts = meta.split_whitespace();
        let (Some(mode), Some(sha), Some(stage)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if mode == "160000" && is_git_hex_oid(sha) {
            if stage == "0" {
                return Some(sha.to_string());
            }
            if fallback.is_none() {
                fallback = Some(sha.to_string());
            }
        }
    }
    fallback
}

/// True when `rel_path` exists in `root` as a reviewable scope entry:
/// either a regular file or a directory backed by a parent index mode `160000` gitlink.
/// Plain directories and missing paths return `false`.
pub fn scope_entry_exists(root: &Path, rel_path: &str) -> bool {
    let Some(norm) = normalize_git_rel_path(rel_path) else {
        return false;
    };
    let full = root.join(&norm);
    if full.is_file() {
        return true;
    }
    if full.is_dir() {
        return gitlink_index_commit(root, &norm).is_some();
    }
    false
}

/// Read canonical scope bytes for `rel_path` under `root`.
///
/// - Regular files take the direct `std::fs::read` fast path (no git subprocesses).
/// - Directories are accepted only when the parent index records mode `160000`:
///   - Uninitialized submodule (no `.git` inside) → `Subproject commit <index_sha>\n`.
///   - Initialized & clean submodule → `Subproject commit <head_sha>\n`.
///   - Initialized & dirty submodule → `Subproject commit <head_sha>-dirty\ndirty-worktree: sha256:<digest>\n`.
///   - Corrupted `.git` inside a 160000 directory → `None` (fail closed).
/// - Plain directories and missing paths return `None`.
pub fn read_scope_bytes(root: &Path, rel_path: &str) -> Option<Vec<u8>> {
    let norm = normalize_git_rel_path(rel_path)?;
    let full = root.join(&norm);
    if !full.is_dir() {
        return std::fs::read(&full).ok();
    }
    let index_sha = gitlink_index_commit(root, &norm)?;
    // 未初始化子模块：目录内无 `.git`，回退父仓索引记录的 160000 SHA
    if full.join(".git").symlink_metadata().is_err() {
        return Some(gitlink_canonical_bytes(&index_sha));
    }
    // 已存在 `.git` 但非该目录自有有效 git 工作树（如损坏 `.git` 文件、空 `.git` 目录
    // 或悬空符号链接）：防止向上爬升命中父仓 `.git`，直接 fail closed
    if !is_own_git_worktree(&full) {
        return None;
    }
    let head_res = git(&full, &["rev-parse", "--verify", "HEAD"]);
    let head_sha = head_res.clone().unwrap_or(index_sha);
    let status_raw = git_bytes_opt(
        &full,
        &["-c", "core.quotepath=false", "status", "--porcelain", "-uall", "-z"],
    )?;
    if status_raw.is_empty() {
        return Some(gitlink_canonical_bytes(&head_sha));
    }
    let digest = submodule_dirty_digest(&full, head_res.is_some(), &status_raw)?;
    Some(format!("Subproject commit {head_sha}-dirty\ndirty-worktree: sha256:{digest}\n").into_bytes())
}

/// Compute a deterministic SHA-256 hex digest over a dirty submodule worktree:
/// raw `status --porcelain -uall -z`, `diff HEAD --` (or staged+unstaged for unborn HEAD),
/// and sorted untracked file paths + raw contents.
fn submodule_dirty_digest(sub_dir: &Path, has_head: bool, status_raw: &[u8]) -> Option<String> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(b"status\0");
    hasher.update((status_raw.len() as u64).to_be_bytes());
    hasher.update(b"\0");
    hasher.update(status_raw);

    hasher.update(b"diff\0");
    if has_head {
        let diff_raw = git_bytes_opt(
            sub_dir,
            &[
                "-c",
                "core.quotepath=false",
                "diff",
                "--no-color",
                "--no-ext-diff",
                "--full-index",
                "HEAD",
                "--",
            ],
        )?;
        hasher.update((diff_raw.len() as u64).to_be_bytes());
        hasher.update(b"\0");
        hasher.update(&diff_raw);
    } else {
        let cached = git_bytes_opt(
            sub_dir,
            &["-c", "core.quotepath=false", "diff", "--no-color", "--no-ext-diff", "--full-index", "--cached"],
        )?;
        let unstaged = git_bytes_opt(
            sub_dir,
            &["-c", "core.quotepath=false", "diff", "--no-color", "--no-ext-diff", "--full-index"],
        )?;
        hasher.update((cached.len() as u64).to_be_bytes());
        hasher.update(b"\0");
        hasher.update(&cached);
        hasher.update((unstaged.len() as u64).to_be_bytes());
        hasher.update(b"\0");
        hasher.update(&unstaged);
    }

    let mut untracked: Vec<String> = Vec::new();
    let mut parts = status_raw.split(|&b| b == 0);
    while let Some(entry) = parts.next() {
        if entry.len() < 4 {
            continue;
        }
        if entry.starts_with(b"?? ") {
            let path = String::from_utf8_lossy(&entry[3..]).replace('\\', "/");
            if !path.is_empty() {
                untracked.push(path);
            }
        } else if matches!(entry[0], b'R' | b'C') || matches!(entry[1], b'R' | b'C') {
            // `-z` 在重命名/复制条目后紧跟原始路径，需要消费掉以免误解析
            let _ = parts.next();
        }
    }
    untracked.sort();
    untracked.dedup();

    hasher.update(b"untracked\0");
    for rel in untracked {
        if let Ok(bytes) = std::fs::read(sub_dir.join(&rel)) {
            hasher.update(rel.as_bytes());
            hasher.update(b"\0");
            hasher.update((bytes.len() as u64).to_be_bytes());
            hasher.update(b"\0");
            hasher.update(&bytes);
            hasher.update(b"\0");
        }
    }

    Some(format!("{:x}", hasher.finalize()))
}

/// 看板排序鍵（board_rank）的合法值：非空、僅小寫 ASCII 英文字母。
/// 邊界驗證擋掉會破壞 meta/frontmatter 解析的值（換行注入、YAML 指示字元）。
pub(crate) fn is_valid_board_rank(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase())
}

/// Write a file, creating parent directories as needed. Content is written verbatim.
///
/// The write lands atomically: content goes to a temp file in the destination's own
/// directory, which is then renamed onto the destination. A concurrent reader therefore
/// observes either the old document or the new one — never an empty or half-written file.
/// If the rename fails (Windows can refuse it while another process holds the destination
/// open), fall back to writing the destination directly: no worse than before atomicity,
/// rather than turning a platform limit into a failed verb.
pub fn write_file(path: &Path, content: &str) -> std::io::Result<()> {
    // A bare `std::fs::rename` cannot be passed here: its inferred lifetimes are not
    // general enough for the higher-ranked bound the injection point needs.
    write_file_with_rename(path, content, |from, to| std::fs::rename(from, to))
}

/// `write_file` with the rename step injected, so the fallback path can be exercised
/// without a platform that actually refuses the rename.
fn write_file_with_rename(
    path: &Path,
    content: &str,
    rename: impl Fn(&Path, &Path) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let target = resolve_symlink_target(path);
    let target = target.as_path();
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = temp_write_path(target);
    if let Err(e) = std::fs::write(&tmp, content) {
        let _ = std::fs::remove_file(&tmp);
        // 只有權限類失敗（父目錄不可寫但目的檔可寫）退回直寫——行為不劣於
        // 原子化前。其他失敗（磁碟滿、暫存名過長）直接浮出且目的檔不變：
        // 退回的直寫多半同樣會失敗，卻已先 truncate 把目的檔清空。
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            return std::fs::write(target, content);
        }
        return Err(e);
    }
    // 目的檔既有 mode 套回暫存檔（best-effort），rename 後才不會被重設成 umask
    // 預設（0600 的設定檔不該被放寬成 0644）。Windows 不做：唯讀屬性複製到暫存
    // 檔會反過來擋掉失敗路徑上的清理，且該平台無 umask 重設問題（ACL 隨目錄繼承）。
    #[cfg(unix)]
    if let Ok(meta) = std::fs::metadata(target) {
        let _ = std::fs::set_permissions(&tmp, meta.permissions());
    }
    if rename(&tmp, target).is_err() {
        let direct = std::fs::write(target, content);
        let _ = std::fs::remove_file(&tmp);
        return direct;
    }
    Ok(())
}

/// Follow a symlinked destination to its final target. The pre-atomic direct write
/// wrote through the link; the atomic replacement must land on the same target, or
/// the link would silently become a regular file and the topology be lost. Bounded
/// walk; on a cycle the last path wins and gets replaced like any file.
fn resolve_symlink_target(path: &Path) -> PathBuf {
    let mut target = path.to_path_buf();
    for _ in 0..8 {
        let Ok(link) = std::fs::read_link(&target) else {
            break;
        };
        target = if link.is_absolute() {
            link
        } else {
            target.parent().map(|p| p.join(&link)).unwrap_or(link)
        };
    }
    target
}

/// Temp path beside `path`: same directory (so the rename stays within one filesystem
/// and thus atomic), unique per process and per call so concurrent writers never collide
/// on one temp name. Directory enumerators (e.g. `walk_files`) may glimpse the entry in
/// the write-to-rename window; only a crash inside that window leaves one behind, and
/// the `.tmp` tail marks such a leftover for what it is.
fn temp_write_path(path: &Path) -> PathBuf {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.{n}.tmp", std::process::id()));
    path.with_file_name(name)
}

/// Recursively collect all files under `dir` (relative paths are not resolved; returns absolute-ish
/// paths as joined). Symlinks are not followed. Order is sorted for determinism.
pub fn walk_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_inner(dir, &mut out);
    out.sort();
    out
}

fn walk_inner(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_inner(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

/// True if the path exists.
pub fn exists(path: &Path) -> bool {
    path.exists()
}

/// Remove a file (thin wrapper so flow modules stay free of direct std::fs calls).
pub fn remove_file(path: &Path) -> std::io::Result<()> {
    std::fs::remove_file(path)
}

/// True if a file exists and has non-whitespace content.
pub fn has_content(path: &Path) -> bool {
    match std::fs::read_to_string(path) {
        Ok(s) => !s.trim().is_empty(),
        Err(_) => false,
    }
}

/// Build a `git -C <root>` command. On Windows, spawn with CREATE_NO_WINDOW: console
/// programs launched from a GUI process otherwise flash a console window per spawn.
/// All callers run short non-interactive commands (config/status) with piped stdio,
/// which need no console.
fn git_command(root: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(root);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Run `git -C <root> <args...>`, returning trimmed stdout on success.
pub fn git(root: &Path, args: &[&str]) -> Option<String> {
    let output = git_command(root).args(args).output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Run `git -C <root> <args...>`, returning raw (untrimmed) stdout on success. Needed for
/// `status --porcelain`, whose first column may be a significant leading space.
pub fn git_raw(root: &Path, args: &[&str]) -> Option<String> {
    let output = git_command(root).args(args).output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

/// Whether the directory is inside a git work tree.
pub fn git_available(root: &Path) -> bool {
    git(root, &["rev-parse", "--is-inside-work-tree"]).as_deref() == Some("true")
}

/// Resolve `<root>/.git` to its git directory:
/// - Regular repo: `<root>/.git` is a directory.
/// - Submodule or linked worktree: `<root>/.git` is a file containing `gitdir: <path>`.
fn resolve_dot_git(root: &Path) -> Option<PathBuf> {
    let dot_git = root.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    if !dot_git.is_file() {
        return None;
    }
    let content = std::fs::read_to_string(&dot_git).ok()?;
    let raw = content.trim().strip_prefix("gitdir:")?.trim();
    if raw.is_empty() {
        return None;
    }
    let target = PathBuf::from(raw);
    let resolved = if target.is_absolute() {
        target
    } else {
        root.join(target)
    };
    resolved.is_dir().then_some(resolved)
}

/// Return the resolved git directory when `root` is a MAIN git checkout (either a
/// regular repo or a git submodule), and `None` for non-git directories and linked
/// worktrees.
///
/// Per `gitrepository-layout(5)`, a linked worktree's `$GIT_DIR` always contains a
/// `commondir` file pointing back to the shared repository, while a regular repo or
/// submodule main checkout never does. Pure filesystem check — no git subprocess.
pub fn main_git_dir(root: &Path) -> Option<PathBuf> {
    let dir = resolve_dot_git(root)?;
    if dir.join("commondir").is_file() {
        return None;
    }
    Some(dir)
}

/// True when `root` is a linked git worktree (`.git` file pointing to a gitdir that
/// carries a `commondir` file); false for both regular main checkouts and submodules.
pub fn is_linked_worktree(root: &Path) -> bool {
    let dot_git = root.join(".git");
    if !dot_git.is_file() {
        return false;
    }
    resolve_dot_git(root).is_some_and(|dir| dir.join("commondir").is_file())
}

/// Today's date as YYYY-MM-DD (local time).
pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// One `Local::now()` rendered twice. archive names the dated directory with `date` and
/// writes `rfc3339` into `@trace updated`, so the two can never straddle midnight.
pub struct NowStamps {
    /// `YYYY-MM-DD` in local time.
    pub date: String,
    /// RFC 3339 with the local offset at second precision, e.g. `2026-09-05T23:17:28+08:00`.
    pub rfc3339: String,
}

pub fn now_stamps() -> NowStamps {
    let now = chrono::Local::now();
    NowStamps {
        date: now.format("%Y-%m-%d").to_string(),
        rfc3339: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
    }
}

/// The time every change metadata field is written with (`created`, `started_at`,
/// `claimed_at`, the station stamps): the RFC 3339 half of [`now_stamps`].
pub fn now_rfc3339() -> String {
    now_stamps().rfc3339
}

/// Test helper: the shape [`now_rfc3339`] writes — RFC 3339 with an explicit
/// offset, second precision.
#[cfg(test)]
pub(crate) fn is_offset_stamp(value: &str) -> bool {
    regex::Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[+-]\d{2}:\d{2}$")
        .unwrap()
        .is_match(value)
}

/// Test helper: the value of a metadata document's `key: ` line, asserted to have
/// the [`now_rfc3339`] shape.
#[cfg(test)]
pub(crate) fn offset_stamp_line<'a>(meta: &'a str, key: &str) -> &'a str {
    let prefix = format!("{key}: ");
    let value = meta
        .lines()
        .find_map(|l| l.strip_prefix(prefix.as_str()))
        .unwrap_or_else(|| panic!("no {key} line: {meta}"));
    assert!(is_offset_stamp(value), "{key} is not an offset stamp: {meta}");
    value
}

/// 剝除 `YYYY-MM-DD-` 日期前綴（封存 change 目錄名、封存後的討論儲存名）；
/// 不合格式原樣回傳（寬容）。共用於 discuss 的 promote 還原與 trace 的
/// 鏈組裝。
pub(crate) fn strip_date_prefix(name: &str) -> &str {
    let b = name.as_bytes();
    let dated = b.len() > 11
        && b[10] == b'-'
        && b[..10]
            .iter()
            .enumerate()
            .all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() });
    if dated {
        &name[11..]
    } else {
        name
    }
}

/// Render a string as a YAML scalar safe to append to a metadata document.
///
/// A newline inside an identity/agent/path string would inject arbitrary YAML
/// fields; a bare `:`/leading indicator would break the whole document's parse
/// (which silently falls back to defaults). Flatten control characters and
/// double-quote values a plain scalar cannot carry.
pub fn yaml_scalar(s: &str) -> String {
    let flat = s.replace(|c: char| c.is_control(), " ");
    let risky = flat.is_empty()
        || flat.contains([':', '#', '"'])
        || flat.ends_with(' ')
        || flat.starts_with([' ', '[', '{', '\'', '&', '*', '!', '|', '>', '%', '@', '`', '-', '?']);
    if risky {
        format!("\"{}\"", flat.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        flat
    }
}

/// [`yaml_scalar`]，再替「讀回來會變成別種值」的字補上引號：YAML 會把 `null`、`~`、
/// `true`、`123`、`1e5` 這類沒加引號的字讀成別種型別（或讀成缺席）。給值域是任意識別字
/// 的欄位用（分支名稱、commit）。它管不到 YAML 根本帶不了的字元（例如 U+FFFE）：值來自
/// 外部時，呼叫端要自己在寫入前確認讀得回來。
pub fn yaml_roundtrip_scalar(s: &str) -> String {
    let scalar = yaml_scalar(s);
    if scalar.starts_with('"') {
        return scalar;
    }
    match serde_yaml::from_str::<serde_yaml::Value>(&scalar) {
        Ok(serde_yaml::Value::String(_)) => scalar,
        _ => format!("\"{}\"", scalar.replace('\\', "\\\\")),
    }
}

/// 一律加雙引號的 YAML 純量：`\\` 與 `"` 跳脫。和 [`yaml_scalar`]、[`yaml_roundtrip_scalar`]
/// 的差別是不看內容決定要不要加引號——值域是任意文字、而且讀回來必須逐字相同的地方用
/// （自由欄位的值：`123`、`true`、`null` 不加引號會被讀成別的型別）。控制字元不壓成空白，
/// 而是和 YAML 帶不了或會當成換行的字元（U+2028、U+2029、U+FFFE、U+FFFF）一起寫成
/// `\uXXXX`，所以任何字串都讀得回來；要不要收控制字元是呼叫端的規則。
pub fn yaml_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}' | '\u{FFFE}' | '\u{FFFF}') => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// [`yaml_scalar`] 的反向：讀回它寫出的值。雙引號包住的值拆掉引號並還原 `\\"`／`\\\\`，
/// 其餘（沒加引號的純量）原樣回傳。給只做逐字讀取的 `KeyLines::get` 呼叫端在「比對
/// 身分」前用——寫入走 `yaml_scalar`，讀回就得走這裡，否則帶引號的本人會被當成他人。
pub fn yaml_unscalar(s: &str) -> String {
    match s.strip_prefix('"').and_then(|rest| rest.strip_suffix('"')) {
        Some(inner) => inner.replace("\\\"", "\"").replace("\\\\", "\\"),
        None => s.to_string(),
    }
}

/// Convert an absolute path to a forward-slash string.
pub fn to_slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Simple time-seeded pseudo-random index in [0, n). Not cryptographic.
pub fn pseudo_random(n: usize) -> usize {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // xorshift-ish mix
    let mut x = (nanos as u64) ^ 0x9E3779B97F4A7C15;
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    if n == 0 {
        0
    } else {
        (x as usize) % n
    }
}

/// Slugify a topic string into kebab-case (lowercase, spaces/underscores → '-', strip others).
pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in input.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if ch == '-' || ch == '_' || ch.is_whitespace() {
            if !prev_dash && !out.is_empty() {
                out.push('-');
                prev_dash = true;
            }
        } else if !ch.is_ascii() {
            // keep unicode letters (e.g., CJK) as-is to preserve meaning
            out.push(ch);
            prev_dash = false;
        }
        // else: drop punctuation
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_quoted_always_quotes_and_reads_back_verbatim() {
        assert_eq!(yaml_quoted("123"), "\"123\"");
        assert_eq!(yaml_quoted(""), "\"\"");
        assert_eq!(yaml_quoted("a \"b\" \\"), "\"a \\\"b\\\" \\\\\"");
        for s in ["123", "true", "null", "~", "a: b # c", "tab\there", "nel\u{85}", "ls\u{2028}ps\u{2029}", "\u{FFFE}\u{FFFF}", "中文 \"x\" \\"] {
            let quoted = yaml_quoted(s);
            assert!(!quoted.contains(['\n', '\r']), "one line for {s:?}");
            assert_eq!(serde_yaml::from_str::<String>(&quoted).unwrap(), s, "round trip for {s:?}");
        }
    }

    #[test]
    fn yaml_unscalar_inverts_yaml_scalar() {
        // 讀回 `yaml_scalar` 寫出的值必須拿回原字串——引號、反斜線、內嵌雙引號都要還原；
        // 沒被引號包住的值原樣回傳。
        for s in [
            "Alice <a@example.com>",
            "Alice: dev <a@example.com>",
            "@alice",
            "-lead",
            "say \"hi\" #1",
            "back\\slash: x",
            "",
        ] {
            assert_eq!(yaml_unscalar(&yaml_scalar(s)), s, "round trip for {s:?}");
        }
        assert_eq!(yaml_unscalar("plain"), "plain");
        assert_eq!(yaml_unscalar("\"unterminated"), "\"unterminated");
    }

    #[test]
    fn now_stamps_share_one_instant() {
        // manual-stale-time-granularity design「archive 以同一個「現在」產生目錄日期前綴與
        // updated 時戳」：時戳為 RFC 3339 帶時區偏移量、秒級，其日曆日就是同時回傳的純日期。
        let NowStamps { date, rfc3339 } = now_stamps();
        let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[+-]\d{2}:\d{2}$").unwrap();
        assert!(re.is_match(&rfc3339), "not an RFC 3339 local stamp: {rfc3339}");
        assert_eq!(&rfc3339[..10], date.as_str(), "stamp and date come from one Local::now()");
        assert_eq!(date.len(), 10);
    }

    /// Throwaway directory in the OS temp dir; removed on drop.
    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> TempDir {
            let path = std::env::temp_dir().join(format!(
                "speclink-core-util-{tag}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            TempDir { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// File names present in a directory, sorted.
    fn dir_entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn write_file_writes_content_verbatim() {
        let d = TempDir::new("verbatim");
        let path = d.path.join("note.md");
        write_file(&path, "line one\nline two\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "line one\nline two\n");
    }

    #[test]
    fn write_file_creates_parent_dirs() {
        let d = TempDir::new("parents");
        let path = d.path.join("a").join("b").join("c.md");
        write_file(&path, "deep").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "deep");
    }

    #[test]
    fn write_file_overwrites_existing() {
        let d = TempDir::new("overwrite");
        let path = d.path.join("note.md");
        write_file(&path, "old content that is longer").unwrap();
        write_file(&path, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }

    /// Spec `本地檔案寫入原子落盤`: the success path leaves no temp file in the
    /// destination directory.
    #[test]
    fn write_file_leaves_no_temp_file_behind() {
        let d = TempDir::new("no-residue");
        let path = d.path.join("note.md");
        write_file(&path, "first").unwrap();
        assert_eq!(dir_entries(&d.path), vec!["note.md".to_string()]);
        write_file(&path, "second").unwrap();
        assert_eq!(dir_entries(&d.path), vec!["note.md".to_string()]);
    }

    /// Symlinked destination keeps its topology: the write lands on the link's
    /// target (as the pre-atomic direct write did), and the link itself survives
    /// instead of being replaced by a regular file.
    #[cfg(unix)]
    #[test]
    fn write_file_writes_through_a_symlinked_destination() {
        let d = TempDir::new("symlink");
        let target = d.path.join("target.md");
        let link = d.path.join("link.md");
        write_file(&target, "old").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        write_file(&link, "new").unwrap();
        assert!(
            std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink(),
            "link must stay a symlink"
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "new");
    }

    /// Overwriting keeps the destination's permission bits (best-effort): a 0600
    /// config must not silently relax to the umask default after the rename.
    #[cfg(unix)]
    #[test]
    fn write_file_preserves_destination_mode() {
        use std::os::unix::fs::PermissionsExt;
        let d = TempDir::new("mode");
        let path = d.path.join("secret.yaml");
        write_file(&path, "old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        write_file(&path, "new").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "mode must survive the atomic replace");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }

    /// Spec `本地檔案寫入原子落盤`: when the temp file cannot even be created (an
    /// unwritable parent dir with a writable destination inside), fall back to the
    /// direct write — behavior no worse than before atomicity.
    #[cfg(unix)]
    #[test]
    fn write_file_falls_back_to_direct_write_when_temp_cannot_be_created() {
        use std::os::unix::fs::PermissionsExt;
        let d = TempDir::new("locked-dir");
        let path = d.path.join("note.md");
        write_file(&path, "old").unwrap();
        std::fs::set_permissions(&d.path, std::fs::Permissions::from_mode(0o555)).unwrap();
        let res = write_file(&path, "new");
        std::fs::set_permissions(&d.path, std::fs::Permissions::from_mode(0o755)).unwrap();
        res.unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(dir_entries(&d.path), vec!["note.md".to_string()]);
    }

    /// Spec `本地檔案寫入原子落盤`: only a permission-class temp failure may fall
    /// back to the direct write. Any other temp-write failure (disk full, a temp
    /// name past NAME_MAX) must surface the error with the destination untouched —
    /// falling back would truncate the destination before failing.
    #[cfg(unix)]
    #[test]
    fn write_file_surfaces_non_permission_temp_failure_without_touching_dest() {
        let d = TempDir::new("temp-name-too-long");
        // 250 bytes: legal for the file itself, but + ".<pid>.<n>.tmp" the temp
        // name passes NAME_MAX (255) and its creation fails with ENAMETOOLONG.
        let long_name = format!("{}.md", "n".repeat(247));
        let path = d.path.join(&long_name);
        // Setup bypasses write_file: its temp name would hit the same limit.
        std::fs::write(&path, "old").unwrap();
        assert!(
            write_file(&path, "new").is_err(),
            "non-permission temp failure must surface"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "old",
            "destination must stay untouched"
        );
    }

    /// Spec `本地檔案寫入原子落盤`: when the rename cannot complete (Windows refuses
    /// it while another process holds the destination open), the content still lands
    /// and the temp file is cleaned up.
    #[test]
    fn write_file_falls_back_to_direct_write_when_rename_fails() {
        let d = TempDir::new("rename-fails");
        let path = d.path.join("note.md");
        write_file(&path, "old").unwrap();

        write_file_with_rename(&path, "new", |_, _| {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "sharing violation",
            ))
        })
        .unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(dir_entries(&d.path), vec!["note.md".to_string()]);
    }

    /// Spec `本地檔案寫入原子落盤` / scenario `並行讀者不見半份內容`: while one
    /// thread rewrites the file, a concurrent reader only ever observes one of
    /// the two complete documents — never an empty, truncated, or mixed read.
    /// Unix only: the atomic guarantee is best-effort on Windows.
    #[cfg(unix)]
    #[test]
    fn write_file_concurrent_reader_never_sees_partial_content() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let d = TempDir::new("concurrent");
        let path = d.path.join("shared.md");
        let doc_a = "a".repeat(64 * 1024);
        let doc_b = "b".repeat(64 * 1024);
        write_file(&path, &doc_a).unwrap();

        let done = Arc::new(AtomicBool::new(false));

        let writer = {
            let path = path.clone();
            let (doc_a, doc_b) = (doc_a.clone(), doc_b.clone());
            let done = Arc::clone(&done);
            std::thread::spawn(move || {
                for i in 0..200 {
                    let doc = if i % 2 == 0 { &doc_b } else { &doc_a };
                    write_file(&path, doc).unwrap();
                }
                done.store(true, Ordering::Relaxed);
            })
        };

        let reader = {
            let path = path.clone();
            let done = Arc::clone(&done);
            std::thread::spawn(move || {
                while !done.load(Ordering::Relaxed) {
                    let got = std::fs::read_to_string(&path).unwrap_or_default();
                    if got != doc_a && got != doc_b {
                        return Some(got.len());
                    }
                }
                None
            })
        };

        writer.join().unwrap();
        let torn = reader.join().unwrap();
        assert!(
            torn.is_none(),
            "reader observed a partial document of {} bytes",
            torn.unwrap_or(0)
        );
    }

    #[test]
    fn main_git_dir_distinguishes_regular_submodule_and_linked_worktree() {
        let d = TempDir::new("git-dir-kind");

        // Non-git directory
        let plain = d.path.join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        assert_eq!(main_git_dir(&plain), None);
        assert!(!is_linked_worktree(&plain));

        // Regular main checkout: `.git/` is a directory
        let regular = d.path.join("regular");
        std::fs::create_dir_all(regular.join(".git")).unwrap();
        assert_eq!(main_git_dir(&regular), Some(regular.join(".git")));
        assert!(!is_linked_worktree(&regular));

        // Submodule main checkout: `.git` file pointing to superproject `.git/modules/sub` (no commondir)
        let sub_gitdir = d.path.join("super").join(".git").join("modules").join("sub");
        let sub = d.path.join("super").join("sub");
        std::fs::create_dir_all(&sub_gitdir).unwrap();
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join(".git"), "gitdir: ../.git/modules/sub\n").unwrap();
        assert_eq!(
            std::fs::canonicalize(main_git_dir(&sub).expect("submodule is a main checkout")).unwrap(),
            std::fs::canonicalize(&sub_gitdir).unwrap()
        );
        assert!(!is_linked_worktree(&sub));

        // Linked worktree of that submodule: gitdir has `commondir`
        let wt_gitdir = sub_gitdir.join("worktrees").join("feat");
        let wt = d.path.join("super").join("sub.worktrees").join("feat");
        std::fs::create_dir_all(&wt_gitdir).unwrap();
        std::fs::create_dir_all(&wt).unwrap();
        std::fs::write(wt_gitdir.join("commondir"), "../..\n").unwrap();
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", wt_gitdir.display())).unwrap();
        assert_eq!(main_git_dir(&wt), None);
        assert!(is_linked_worktree(&wt));
    }

    /// Run a git command in `dir`, panicking with stdout/stderr on failure.
    fn run_git_test(dir: &Path, args: &[&str]) -> String {
        let out = git_command(dir)
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "core.quotepath=false",
            ])
            .args(args)
            .output()
            .expect("git should execute");
        assert!(
            out.status.success(),
            "git {:?} failed in {}:\nstdout: {}\nstderr: {}",
            args,
            dir.display(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// Build a temp parent git repo with a 160000 submodule at `frontend` and a tracked plain dir `plain_dir`.
    fn make_repo_with_submodule(tag: &str) -> (TempDir, PathBuf, String) {
        let d = TempDir::new(tag);
        let upstream = d.path.join("upstream-sub");
        std::fs::create_dir_all(&upstream).unwrap();
        run_git_test(&upstream, &["init", "-b", "main"]);
        std::fs::write(upstream.join("app.txt"), "v1\n").unwrap();
        run_git_test(&upstream, &["add", "app.txt"]);
        run_git_test(&upstream, &["commit", "-m", "init sub"]);
        let sub_sha = run_git_test(&upstream, &["rev-parse", "HEAD"]);

        let parent = d.path.join("parent");
        std::fs::create_dir_all(parent.join("plain_dir")).unwrap();
        run_git_test(&parent, &["init", "-b", "main"]);
        std::fs::write(parent.join("README.md"), "hello\n").unwrap();
        std::fs::write(parent.join("plain_dir/note.txt"), "plain\n").unwrap();
        run_git_test(&parent, &["add", "README.md", "plain_dir/note.txt"]);
        run_git_test(&parent, &["commit", "-m", "init parent"]);

        let upstream_str = upstream.to_str().unwrap();
        run_git_test(
            &parent,
            &["-c", "protocol.file.allow=always", "submodule", "add", upstream_str, "frontend"],
        );
        run_git_test(&parent, &["commit", "-m", "add frontend submodule"]);
        (d, parent, sub_sha)
    }

    #[test]
    fn scope_helpers_regular_file_and_plain_dir() {
        let (_d, parent, _sub_sha) = make_repo_with_submodule("regular-and-dir");

        // Regular file fast path
        assert!(scope_entry_exists(&parent, "README.md"));
        assert_eq!(read_scope_bytes(&parent, "README.md"), Some(b"hello\n".to_vec()));
        assert_eq!(gitlink_commit_at(&parent, "HEAD", "README.md"), None);

        // Missing file
        assert!(!scope_entry_exists(&parent, "missing.txt"));
        assert_eq!(read_scope_bytes(&parent, "missing.txt"), None);
        assert_eq!(gitlink_commit_at(&parent, "HEAD", "missing.txt"), None);

        // Plain directory (non-160000) must fail closed
        assert!(!scope_entry_exists(&parent, "plain_dir"));
        assert_eq!(read_scope_bytes(&parent, "plain_dir"), None);
        assert_eq!(gitlink_commit_at(&parent, "HEAD", "plain_dir"), None);
    }

    #[test]
    fn scope_helpers_clean_and_dirty_submodule() {
        let (_d, parent, sub_sha) = make_repo_with_submodule("clean-and-dirty");
        let sub_dir = parent.join("frontend");

        // Clean submodule
        assert_eq!(gitlink_commit_at(&parent, "HEAD", "frontend"), Some(sub_sha.clone()));
        assert!(scope_entry_exists(&parent, "frontend"));
        let expected_clean = format!("Subproject commit {sub_sha}\n").into_bytes();
        assert_eq!(read_scope_bytes(&parent, "frontend"), Some(expected_clean.clone()));

        // Dirty tracked file in submodule
        std::fs::write(sub_dir.join("app.txt"), "v1-dirty\n").unwrap();
        assert!(scope_entry_exists(&parent, "frontend"));
        let dirty_tracked = read_scope_bytes(&parent, "frontend").expect("dirty bytes");
        let dirty_tracked_str = String::from_utf8(dirty_tracked.clone()).unwrap();
        let expected_prefix = format!("Subproject commit {sub_sha}-dirty\ndirty-worktree: sha256:");
        assert!(
            dirty_tracked_str.starts_with(&expected_prefix) && dirty_tracked_str.ends_with('\n'),
            "unexpected dirty canonical format: {dirty_tracked_str:?}"
        );
        let hex_part = dirty_tracked_str
            .strip_prefix(&expected_prefix)
            .unwrap()
            .strip_suffix('\n')
            .unwrap();
        assert_eq!(hex_part.len(), 64);
        assert!(hex_part.bytes().all(|b| b.is_ascii_hexdigit()));

        // Adding an untracked file must change the digest
        std::fs::write(sub_dir.join("untracked.txt"), "u1\n").unwrap();
        let dirty_u1 = read_scope_bytes(&parent, "frontend").expect("dirty u1");
        assert_ne!(dirty_u1, dirty_tracked);

        // Second edit to the untracked file must change the digest again
        std::fs::write(sub_dir.join("untracked.txt"), "u2\n").unwrap();
        let dirty_u2 = read_scope_bytes(&parent, "frontend").expect("dirty u2");
        assert_ne!(dirty_u2, dirty_u1);

        // Cleaning the submodule restores exact clean canonical bytes
        run_git_test(&sub_dir, &["checkout", "--", "app.txt"]);
        std::fs::remove_file(sub_dir.join("untracked.txt")).unwrap();
        assert_eq!(read_scope_bytes(&parent, "frontend"), Some(expected_clean));
    }

    #[test]
    fn scope_helpers_uninitialized_corrupted_and_deleted_submodule() {
        let (_d, parent, sub_sha) = make_repo_with_submodule("uninit-and-del");
        let sub_dir = parent.join("frontend");

        // Uninitialized submodule: deinit leaves empty dir without `.git`, falls back to index SHA
        run_git_test(&parent, &["submodule", "deinit", "-f", "frontend"]);
        assert!(sub_dir.is_dir());
        assert!(!sub_dir.join(".git").exists());
        assert!(scope_entry_exists(&parent, "frontend"));
        assert_eq!(
            read_scope_bytes(&parent, "frontend"),
            Some(format!("Subproject commit {sub_sha}\n").into_bytes())
        );

        // Corrupted `.git` in 160000 dir: exists (so stamp catches it) but read returns None (fail closed)
        std::fs::write(sub_dir.join(".git"), "gitdir: /nonexistent/broken/gitdir\n").unwrap();
        assert!(scope_entry_exists(&parent, "frontend"));
        assert_eq!(read_scope_bytes(&parent, "frontend"), None);

        // Deleted submodule from worktree and index
        std::fs::remove_file(sub_dir.join(".git")).unwrap();
        run_git_test(&parent, &["rm", "-f", "frontend"]);
        assert!(!scope_entry_exists(&parent, "frontend"));
        assert_eq!(read_scope_bytes(&parent, "frontend"), None);
        // HEAD tree still has the gitlink commit before committing the deletion
        assert_eq!(gitlink_commit_at(&parent, "HEAD", "frontend"), Some(sub_sha));
    }

    #[test]
    fn scope_helpers_sharp_edges_dash_spaces_non_ascii_and_traversal() {
        let d = TempDir::new("sharp-edges");
        let upstream = d.path.join("upstream");
        std::fs::create_dir_all(&upstream).unwrap();
        run_git_test(&upstream, &["init", "-q", "-b", "main"]);
        std::fs::write(upstream.join("app.txt"), "v1\n").unwrap();
        run_git_test(&upstream, &["add", "app.txt"]);
        run_git_test(&upstream, &["commit", "-q", "-m", "init"]);
        let sub_sha = run_git_test(&upstream, &["rev-parse", "HEAD"]);

        let parent = d.path.join("parent");
        std::fs::create_dir_all(&parent).unwrap();
        run_git_test(&parent, &["init", "-q", "-b", "main"]);
        std::fs::write(parent.join("README.md"), "root\n").unwrap();
        run_git_test(&parent, &["add", "README.md"]);
        run_git_test(&parent, &["commit", "-q", "-m", "init"]);

        let src = upstream.to_str().unwrap();
        // Leading `-` gitlink (git's own `submodule add` omits `--` internally, so clone + `git add --` creates the 160000 entry directly)
        run_git_test(
            &parent,
            &["-c", "protocol.file.allow=always", "clone", "-q", src, "--", "-dash-sub"],
        );
        run_git_test(&parent, &["add", "--", "-dash-sub"]);
        // Space + non-ASCII CJK path
        run_git_test(
            &parent,
            &["-c", "protocol.file.allow=always", "submodule", "add", "-q", src, "sub dir/前端"],
        );
        run_git_test(&parent, &["commit", "-q", "-m", "add edge submodules"]);

        let expected = format!("Subproject commit {sub_sha}\n").into_bytes();
        for mount in ["-dash-sub", "sub dir/前端"] {
            assert_eq!(gitlink_commit_at(&parent, "HEAD", mount), Some(sub_sha.clone()), "{mount}");
            assert!(scope_entry_exists(&parent, mount), "{mount}");
            assert_eq!(read_scope_bytes(&parent, mount), Some(expected.clone()), "{mount}");

            // Dirty with non-ASCII untracked file
            std::fs::write(parent.join(mount).join("新增 檔案.txt"), "dirty\n").unwrap();
            let dirty = read_scope_bytes(&parent, mount).expect("dirty bytes");
            assert_ne!(dirty, expected, "{mount} dirty bytes must differ");
        }

        // Reject traversal, absolute paths, NUL bytes, and flag-like revisions
        for evil in ["../upstream/app.txt", "/etc/passwd", "a\0b", "C:evil", "", "."] {
            assert!(!scope_entry_exists(&parent, evil), "must reject {evil:?}");
            assert_eq!(read_scope_bytes(&parent, evil), None, "must reject {evil:?}");
            assert_eq!(gitlink_commit_at(&parent, "HEAD", evil), None, "must reject {evil:?}");
        }
        assert_eq!(gitlink_commit_at(&parent, "--help", "-dash-sub"), None);

        // Collapse `./` and repeated `/` so lookups match ls-tree / ls-files canonical paths
        assert_eq!(gitlink_commit_at(&parent, "HEAD", "./-dash-sub"), Some(sub_sha.clone()));
        assert_eq!(gitlink_commit_at(&parent, "HEAD", "sub dir//前端"), Some(sub_sha.clone()));
        assert!(scope_entry_exists(&parent, "./sub dir//前端"));

        // File literally named `HEAD` inside dirty submodule must not trigger git rev/path ambiguity
        std::fs::write(parent.join("-dash-sub").join("HEAD"), "not a ref\n").unwrap();
        assert!(
            read_scope_bytes(&parent, "-dash-sub").is_some(),
            "dirty submodule with file named HEAD must still hash cleanly"
        );
    }

    #[test]
    fn scope_helpers_corrupted_dot_git_dir_and_dangling_symlink_do_not_climb_to_parent() {
        let (_d, parent, _sub_sha) = make_repo_with_submodule("corrupt-climb");
        let sub_dir = parent.join("frontend");
        run_git_test(&parent, &["submodule", "deinit", "-f", "frontend"]);

        // Empty `.git/` directory inside 160000 dir must fail closed, never read parent `.git`
        std::fs::create_dir(sub_dir.join(".git")).unwrap();
        assert!(scope_entry_exists(&parent, "frontend"));
        assert_eq!(read_scope_bytes(&parent, "frontend"), None);
        std::fs::remove_dir(sub_dir.join(".git")).unwrap();

        // Dangling `.git` symlink must fail closed, never climb to parent `.git`
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/definitely/missing/gitdir", sub_dir.join(".git")).unwrap();
            assert!(scope_entry_exists(&parent, "frontend"));
            assert_eq!(read_scope_bytes(&parent, "frontend"), None);
        }
    }
}
