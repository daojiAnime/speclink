use std::borrow::Cow;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve the compile-time Git revision for `manifest_dir` and emit Cargo
/// `rerun-if-changed` directives for Git metadata and all tracked workspace files.
// Called by `build.rs` and `tests/it/engine_version.rs`; `main.rs` mounts this module under `#[cfg(test)]`.
#[allow(dead_code)]
pub fn resolve_for_manifest(manifest_dir: &Path, emit_watch: &mut dyn FnMut(&Path)) -> Option<String> {
    let workspace_root = manifest_dir.parent()?.parent()?.parent()?;
    resolve_git_rev_with(manifest_dir, workspace_root, OsStr::new("git"), emit_watch)
}

/// Resolve the 8-char lowercase hex commit SHA (plus `-dirty` when tracked
/// workspace files are modified) only when `manifest_dir` belongs to a Git
/// repository whose top-level matches `workspace_root`.
///
/// Canonicalized paths are used solely for equality comparison; subsequent
/// `git -C` calls and path joins use the un-canonicalized `workspace_root` to
/// avoid Windows `\\?\` verbatim path prefix issues with `/` separators.
pub fn resolve_git_rev_with(
    manifest_dir: &Path,
    workspace_root: &Path,
    git_bin: &OsStr,
    emit_watch: &mut dyn FnMut(&Path),
) -> Option<String> {
    let toplevel = git_stdout(
        manifest_dir,
        git_bin,
        &["-c", "core.quotepath=false", "rev-parse", "--show-toplevel"],
    )?;
    if fs::canonicalize(toplevel.trim()).ok()? != fs::canonicalize(workspace_root).ok()? {
        return None;
    }

    watch_git_metadata(workspace_root, git_bin, emit_watch);
    watch_tracked_files(workspace_root, git_bin, emit_watch);

    let sha = git_stdout(workspace_root, git_bin, &["rev-parse", "--short=8", "HEAD"])?;
    let sha = sha.trim();
    if sha.len() < 8 || !sha.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let sha8 = &sha[..8];

    // Check emptiness directly on raw bytes so non-UTF-8 tracked paths still mark -dirty.
    let status_bytes = git_stdout_bytes(
        workspace_root,
        git_bin,
        &[
            "-c",
            "core.quotepath=false",
            "status",
            "--porcelain",
            "--untracked-files=no",
        ],
    )?;
    Some(if status_bytes.is_empty() {
        sha8.to_string()
    } else {
        format!("{sha8}-dirty")
    })
}

fn watch_git_metadata(root: &Path, git_bin: &OsStr, emit_watch: &mut dyn FnMut(&Path)) {
    let symref = git_stdout(root, git_bin, &["symbolic-ref", "-q", "HEAD"])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let mut args = vec![
        "-c",
        "core.quotepath=false",
        "rev-parse",
        "--git-path",
        "HEAD",
        "--git-path",
        "packed-refs",
        "--git-path",
        "index",
        "--git-common-dir",
    ];
    let expected_lines = if let Some(ref sym) = symref {
        args.extend(["--git-path", sym.as_str()]);
        5
    } else {
        4
    };

    let Some(paths_out) = git_stdout(root, git_bin, &args) else {
        return;
    };
    let lines: Vec<&str> = paths_out.lines().map(str::trim).collect();
    if lines.len() != expected_lines || lines.iter().any(|l| l.is_empty()) {
        return;
    }

    for rel in &lines[..3] {
        watch_if_exists(&join_git_rel(root, rel), emit_watch);
    }
    if let Some(sym_rel) = lines.get(4) {
        let common_dir = join_git_rel(root, lines[3]);
        let mut seen = HashSet::new();
        watch_path_or_existing_parent(
            &join_git_rel(root, sym_rel),
            &common_dir,
            &mut seen,
            emit_watch,
        );
    }
}

fn watch_tracked_files(root: &Path, git_bin: &OsStr, emit_watch: &mut dyn FnMut(&Path)) {
    let Some(bytes) = git_stdout_bytes(
        root,
        git_bin,
        &["-c", "core.quotepath=false", "ls-files", "-z"],
    ) else {
        return;
    };
    let mut seen = HashSet::new();
    for entry in bytes.split(|&b| b == 0) {
        if entry.is_empty() {
            continue;
        }
        if let Ok(rel) = std::str::from_utf8(entry) {
            let path = join_git_rel(root, rel);
            watch_path_or_existing_parent(&path, root, &mut seen, emit_watch);
            // Submodule (mode 160000) directory mtime alone misses inner commits or staged state.
            if path.is_dir() && path.join(".git").exists() {
                watch_submodule_metadata(&path, git_bin, &mut seen, emit_watch);
            }
        }
    }
}

fn watch_submodule_metadata(
    sub_root: &Path,
    git_bin: &OsStr,
    seen: &mut HashSet<PathBuf>,
    emit_watch: &mut dyn FnMut(&Path),
) {
    let Some(paths_out) = git_stdout(
        sub_root,
        git_bin,
        &[
            "-c",
            "core.quotepath=false",
            "rev-parse",
            "--git-path",
            "HEAD",
            "--git-path",
            "index",
        ],
    ) else {
        return;
    };
    for rel in paths_out.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let path = join_git_rel(sub_root, rel);
        if path.exists() && seen.insert(path.clone()) {
            emit_watch_safe(&path, emit_watch);
        }
    }
}

fn join_git_rel(root: &Path, rel: &str) -> PathBuf {
    let normalized = if cfg!(windows) {
        Cow::Owned(rel.replace('/', "\\"))
    } else {
        Cow::Borrowed(rel)
    };
    let candidate = Path::new(normalized.as_ref());
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    }
}

fn watch_if_exists(path: &Path, emit_watch: &mut dyn FnMut(&Path)) {
    if path.exists() {
        emit_watch_safe(path, emit_watch);
    }
}

/// Watch `path` when present, or its nearest existing ancestor strictly below
/// `stop_at`, so deleted tracked files and packed refs still retrigger builds
/// without ever emitting `workspace_root` (which would scan `target/`).
fn watch_path_or_existing_parent(
    path: &Path,
    stop_at: &Path,
    seen: &mut HashSet<PathBuf>,
    emit_watch: &mut dyn FnMut(&Path),
) {
    let mut cur = Some(path);
    while let Some(candidate) = cur {
        if candidate == stop_at {
            return;
        }
        if candidate.exists() {
            if seen.insert(candidate.to_path_buf()) {
                emit_watch_safe(candidate, emit_watch);
            }
            return;
        }
        cur = candidate.parent();
    }
}

fn emit_watch_safe(path: &Path, emit_watch: &mut dyn FnMut(&Path)) {
    if let Some(display) = path.to_str() {
        if !display.contains('\n') && !display.contains('\r') {
            emit_watch(path);
        }
    }
}

fn git_stdout(dir: &Path, git_bin: &OsStr, args: &[&str]) -> Option<String> {
    let bytes = git_stdout_bytes(dir, git_bin, args)?;
    String::from_utf8(bytes).ok()
}

fn git_command(dir: &Path, git_bin: &OsStr) -> Command {
    let mut cmd = Command::new(git_bin);
    cmd.env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        // Avoid `.git/index.lock` contention with concurrent git invocations during `status`.
        .env("GIT_OPTIONAL_LOCKS", "0")
        .arg("-C")
        .arg(dir);
    cmd
}

fn git_stdout_bytes(dir: &Path, git_bin: &OsStr, args: &[&str]) -> Option<Vec<u8>> {
    let out = git_command(dir, git_bin).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(out.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDirGuard(PathBuf);

    impl TempDirGuard {
        fn new(label: &str) -> Self {
            static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let id = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "speclink-git-rev-{label}-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn run_git(dir: &Path, args: &[&str]) -> String {
        let out = git_command(dir, OsStr::new("git"))
            .args(args)
            .output()
            .expect("run git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).to_string()
    }

    fn init_repo_with_commit(dir: &Path) -> (PathBuf, String) {
        run_git(dir, &["init"]);
        run_git(dir, &["config", "user.name", "Test"]);
        run_git(dir, &["config", "user.email", "test@example.com"]);
        let manifest_dir = dir.join("crates/adapters/speclink-cli");
        fs::create_dir_all(&manifest_dir).unwrap();
        fs::create_dir_all(dir.join("docs")).unwrap();
        fs::write(manifest_dir.join("Cargo.toml"), "[package]\n").unwrap();
        fs::write(dir.join("docs/guide.md"), "# Guide\n").unwrap();
        run_git(dir, &["add", "."]);
        run_git(dir, &["commit", "-m", "init"]);
        let sha8 = run_git(dir, &["rev-parse", "--short=8", "HEAD"])
            .trim()[..8]
            .to_string();
        (manifest_dir, sha8)
    }

    #[test]
    fn untracked_files_stay_clean_and_watches_include_non_crates_files() {
        let guard = TempDirGuard::new("untracked");
        let root = &guard.0;
        let (manifest_dir, sha8) = init_repo_with_commit(root);
        fs::write(root.join("scratch.txt"), "untracked\n").unwrap();

        let mut watches = Vec::new();
        let rev = resolve_git_rev_with(&manifest_dir, root, OsStr::new("git"), &mut |p| {
            watches.push(p.to_path_buf());
        });
        assert_eq!(rev.as_deref(), Some(sha8.as_str()));
        assert!(watches.iter().any(|p| p == &root.join("docs/guide.md")));
    }

    #[test]
    fn unstaged_and_staged_modifications_outside_crates_mark_dirty() {
        let guard = TempDirGuard::new("dirty");
        let root = &guard.0;
        let (manifest_dir, sha8) = init_repo_with_commit(root);
        let expected_dirty = format!("{sha8}-dirty");

        fs::write(root.join("docs/guide.md"), "# Modified\n").unwrap();
        assert_eq!(
            resolve_git_rev_with(&manifest_dir, root, OsStr::new("git"), &mut |_| {}),
            Some(expected_dirty.clone())
        );

        run_git(root, &["add", "docs/guide.md"]);
        assert_eq!(
            resolve_git_rev_with(&manifest_dir, root, OsStr::new("git"), &mut |_| {}),
            Some(expected_dirty)
        );
    }

    #[test]
    fn deleted_tracked_file_watches_existing_parent_without_emitting_workspace_root() {
        let guard = TempDirGuard::new("deleted-tracked");
        let root = &guard.0;
        let (manifest_dir, _) = init_repo_with_commit(root);

        // Nested deletion keeps an existing subdirectory parent -> watches `docs/`, never `root`.
        fs::create_dir_all(root.join("docs/sub")).unwrap();
        fs::write(root.join("docs/sub/a.md"), "a\n").unwrap();
        fs::write(root.join("docs/sub/b.md"), "b\n").unwrap();
        run_git(root, &["add", "docs/sub"]);
        run_git(root, &["commit", "-m", "nested"]);
        let sha_nested = run_git(root, &["rev-parse", "--short=8", "HEAD"])
            .trim()[..8]
            .to_string();
        fs::remove_dir_all(root.join("docs/sub")).unwrap();

        let mut nested_watches = Vec::new();
        let rev = resolve_git_rev_with(&manifest_dir, root, OsStr::new("git"), &mut |p| {
            nested_watches.push(p.to_path_buf());
        });
        assert_eq!(rev.as_deref(), Some(format!("{sha_nested}-dirty").as_str()));
        assert_eq!(
            nested_watches.iter().filter(|p| *p == &root.join("docs")).count(),
            1,
            "missing files under the same deleted subdir should deduplicate parent watch"
        );
        assert!(!nested_watches.iter().any(|p| p == root));
        assert!(nested_watches.iter().all(|p| p.exists()));

        // Top-level directory deletion must not emit `workspace_root` (would scan `target/`).
        fs::remove_dir_all(root.join("docs")).unwrap();
        let mut root_watches = Vec::new();
        let rev2 = resolve_git_rev_with(&manifest_dir, root, OsStr::new("git"), &mut |p| {
            root_watches.push(p.to_path_buf());
        });
        assert_eq!(rev2.as_deref(), Some(format!("{sha_nested}-dirty").as_str()));
        assert!(!root_watches.iter().any(|p| p == root));
        assert!(root_watches.iter().all(|p| p.exists()));
    }

    #[test]
    fn linked_worktree_with_packed_nested_branch_resolves_and_watches_existing_paths() {
        let repo_guard = TempDirGuard::new("wt-main");
        let root = &repo_guard.0;
        let (_, sha8) = init_repo_with_commit(root);
        run_git(root, &["branch", "feat/nested"]);
        run_git(root, &["pack-refs", "--all"]);

        let wt_guard = TempDirGuard::new("wt-linked");
        let wt_root = wt_guard.0.join("wt");
        run_git(root, &["worktree", "add", wt_root.to_str().unwrap(), "feat/nested"]);

        let mut watches = Vec::new();
        let rev = resolve_git_rev_with(
            &wt_root.join("crates/adapters/speclink-cli"),
            &wt_root,
            OsStr::new("git"),
            &mut |p| watches.push(p.to_path_buf()),
        );
        assert_eq!(rev.as_deref(), Some(sha8.as_str()));
        assert!(!watches.is_empty() && watches.iter().all(|p| p.exists()));
    }

    #[test]
    fn missing_git_non_git_empty_repo_and_outer_repo_fall_back_to_none() {
        let repo_guard = TempDirGuard::new("fallback-repo");
        let root = &repo_guard.0;
        let (manifest_dir, _) = init_repo_with_commit(root);

        // Missing git binary.
        assert_eq!(
            resolve_git_rev_with(
                &manifest_dir,
                root,
                OsStr::new("/nonexistent-git-binary-for-speclink-test"),
                &mut |_| {},
            ),
            None
        );

        // Unrelated parent Git repo (e.g. extracted tarball placed inside another repo).
        let nested_manifest = root.join("extracted-tarball/crates/adapters/speclink-cli");
        fs::create_dir_all(&nested_manifest).unwrap();
        assert_eq!(
            resolve_git_rev_with(
                &nested_manifest,
                &root.join("extracted-tarball"),
                OsStr::new("git"),
                &mut |_| {},
            ),
            None
        );

        // Plain non-Git directory.
        let plain_guard = TempDirGuard::new("plain");
        let plain_manifest = plain_guard.0.join("crates/adapters/speclink-cli");
        fs::create_dir_all(&plain_manifest).unwrap();
        assert_eq!(
            resolve_git_rev_with(&plain_manifest, &plain_guard.0, OsStr::new("git"), &mut |_| {}),
            None
        );

        // Empty Git repo (no commits yet).
        let empty_guard = TempDirGuard::new("empty");
        run_git(&empty_guard.0, &["init"]);
        let empty_manifest = empty_guard.0.join("crates/adapters/speclink-cli");
        fs::create_dir_all(&empty_manifest).unwrap();
        assert_eq!(
            resolve_git_rev_with(&empty_manifest, &empty_guard.0, OsStr::new("git"), &mut |_| {}),
            None
        );
    }

    #[test]
    fn initialized_submodule_marks_dirty_and_watches_submodule_git_metadata() {
        let sub_guard = TempDirGuard::new("sub-upstream");
        let sub_dir = &sub_guard.0;
        run_git(sub_dir, &["init", "-b", "main"]);
        run_git(sub_dir, &["config", "user.name", "Test"]);
        run_git(sub_dir, &["config", "user.email", "test@example.com"]);
        fs::write(sub_dir.join("app.txt"), "v1\n").unwrap();
        run_git(sub_dir, &["add", "app.txt"]);
        run_git(sub_dir, &["commit", "-m", "sub init"]);

        let repo_guard = TempDirGuard::new("with-sub");
        let root = &repo_guard.0;
        let (manifest_dir, _) = init_repo_with_commit(root);
        run_git(
            root,
            &[
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "add",
                sub_dir.to_str().unwrap(),
                "frontend",
            ],
        );
        run_git(root, &["commit", "-m", "add submodule"]);
        let sha8 = run_git(root, &["rev-parse", "--short=8", "HEAD"])
            .trim()[..8]
            .to_string();

        let mut watches = Vec::new();
        let rev = resolve_git_rev_with(&manifest_dir, root, OsStr::new("git"), &mut |p| {
            watches.push(p.to_path_buf());
        });
        assert_eq!(rev.as_deref(), Some(sha8.as_str()));
        assert!(
            watches.iter().any(|p| p.ends_with("modules/frontend/HEAD") || p.ends_with("frontend/.git/HEAD")),
            "expected submodule HEAD watch, got {watches:?}"
        );

        fs::write(root.join("frontend/app.txt"), "dirty\n").unwrap();
        let dirty_rev = resolve_git_rev_with(&manifest_dir, root, OsStr::new("git"), &mut |_| {});
        assert_eq!(dirty_rev.as_deref(), Some(format!("{sha8}-dirty").as_str()));
    }
}
