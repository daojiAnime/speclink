//! Integration tests for `speclink worktree status` and `speclink worktree preflight`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct TempRepo {
    base: PathBuf,
    main: PathBuf,
    sub_upstream: Option<PathBuf>,
}

impl TempRepo {
    fn new(tag: &str) -> Self {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!(
            "speclink-cli-wt-verbs-{tag}-{}-{id}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let main = base.join("main");
        std::fs::create_dir_all(main.join("speclink/specs/auth")).unwrap();
        std::fs::write(
            main.join("speclink/specs/auth/spec.md"),
            "# auth\n\n## Purpose\nAuth\n\n## Requirements\n\n### Requirement: Login\nShall login.\n",
        )
        .unwrap();
        std::fs::write(
            main.join("speclink/config.yaml"),
            "schema: spec-driven\nworktree: true\n",
        )
        .unwrap();
        std::fs::create_dir_all(main.join("src")).unwrap();
        std::fs::write(main.join("src/lib.rs"), "pub fn base() {}\n").unwrap();
        git_in(&main, &["init", "-q", "-b", "main"]);
        git_in(&main, &["config", "user.name", "Sandbox Tester"]);
        git_in(&main, &["config", "user.email", "sandbox@example.com"]);
        git_in(&main, &["add", "-A"]);
        git_in(&main, &["commit", "-q", "-m", "init"]);
        Self { base, main, sub_upstream: None }
    }

    fn add_submodule(&mut self, name: &str) {
        let sub_up = self.base.join(format!("{name}-upstream"));
        std::fs::create_dir_all(&sub_up).unwrap();
        std::fs::write(sub_up.join("app.txt"), "v1\n").unwrap();
        git_in(&sub_up, &["init", "-q", "-b", "main"]);
        git_in(&sub_up, &["config", "user.name", "Sandbox Tester"]);
        git_in(&sub_up, &["config", "user.email", "sandbox@example.com"]);
        git_in(&sub_up, &["add", "-A"]);
        git_in(&sub_up, &["commit", "-q", "-m", "sub init"]);
        git_in(
            &self.main,
            &[
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "add",
                "-q",
                sub_up.to_str().unwrap(),
                name,
            ],
        );
        git_in(&self.main, &["commit", "-q", "-m", "add submodule"]);
        self.sub_upstream = Some(sub_up);
    }

    fn put_change(&self, name: &str, cap: &str) {
        let dir = self.main.join("speclink/changes").join(name);
        std::fs::create_dir_all(dir.join("specs").join(cap)).unwrap();
        std::fs::write(dir.join(".meta.yaml"), "schema: spec-driven\ncreated: 2026-11-28\n")
            .unwrap();
        std::fs::write(dir.join("proposal.md"), "## Why\nFix.\n\n## What Changes\n- x\n")
            .unwrap();
        std::fs::write(
            dir.join("specs").join(cap).join("spec.md"),
            "## ADDED Requirements\n\n### Requirement: Extra\nShall work.\n",
        )
        .unwrap();
        std::fs::write(dir.join("tasks.md"), "- [x] 1.1 done\n").unwrap();
    }

    fn add_worktree(&self, change: &str) -> PathBuf {
        let wt = self.base.join(format!("wt-{change}"));
        let branch = format!("speclink/{change}");
        git_in(
            &self.main,
            &["worktree", "add", "-q", "-b", &branch, wt.to_str().unwrap()],
        );
        if self.sub_upstream.is_some() {
            git_in(
                &wt,
                &["-c", "protocol.file.allow=always", "submodule", "update", "--init", "-q"],
            );
        }
        wt
    }

    fn run_in(&self, cwd: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_speclink"))
            .args(args)
            .current_dir(cwd)
            .env("HOME", &self.base)
            .env("USERPROFILE", &self.base)
            .env("XDG_CONFIG_HOME", &self.base)
            .output()
            .expect("run speclink")
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

fn git_in(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "core.quotepath=false"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run git");
    assert!(
        out.status.success(),
        "git {args:?} failed in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn json_of(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "invalid json: {e}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

#[test]
fn worktree_status_and_preflight_surface_landed_meta_overlaps_and_submodule_divergence() {
    let mut r = TempRepo::new("sub-overlap");
    r.add_submodule("frontend");
    r.put_change("first", "auth");
    r.put_change("second", "auth");
    r.put_change("prev-landed", "other");
    git_in(&r.main, &["add", "-A"]);
    git_in(&r.main, &["commit", "-q", "-m", "seed"]);

    let wt1 = r.add_worktree("first");
    let wt2 = r.add_worktree("second");

    std::fs::write(wt1.join("frontend/app.txt"), "v1\nwt1\n").unwrap();
    git_in(&wt1.join("frontend"), &["add", "app.txt"]);
    git_in(&wt1.join("frontend"), &["commit", "-q", "-m", "wt1 sub"]);
    git_in(&wt1, &["add", "frontend"]);
    git_in(&wt1, &["commit", "-q", "-m", "wt1 gitlink"]);

    std::fs::write(r.main.join("frontend/app.txt"), "main\nv1\n").unwrap();
    git_in(&r.main.join("frontend"), &["add", "app.txt"]);
    git_in(&r.main.join("frontend"), &["commit", "-q", "-m", "main sub"]);
    git_in(&r.main, &["add", "frontend"]);
    git_in(&r.main, &["commit", "-q", "-m", "main gitlink"]);

    std::fs::write(wt2.join("frontend/app.txt"), "v1\nwt2\n").unwrap();
    git_in(&wt2.join("frontend"), &["add", "app.txt"]);
    git_in(&wt2.join("frontend"), &["commit", "-q", "-m", "wt2 sub"]);
    git_in(&wt2, &["add", "frontend"]);
    git_in(&wt2, &["commit", "-q", "-m", "wt2 gitlink"]);

    let head = git_in(&r.main, &["rev-parse", "HEAD"]).trim().to_string();
    std::fs::write(
        r.main.join("speclink/changes/prev-landed/.meta.yaml"),
        format!("schema: spec-driven\ncreated: 2026-11-28\nlanded_commit: {head}\n"),
    )
    .unwrap();

    // Status invoked from inside a linked worktree still discovers main + siblings.
    let status_wt = r.run_in(&wt1, &["worktree", "status", "--json"]);
    assert!(
        status_wt.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&status_wt.stderr)
    );
    let sj = json_of(&status_wt);
    assert_eq!(sj["inLinkedWorktree"], true);
    assert_eq!(sj["main"]["cleanExceptLandedMeta"], true);
    assert_eq!(sj["main"]["dirtyLandedMeta"][0]["change"], "prev-landed");
    assert!(sj["wave1Overlaps"].as_array().unwrap().len() >= 2);

    // Human output with --no-color emits zero ANSI escape sequences.
    let human = r.run_in(&r.main, &["worktree", "status", "--no-color"]);
    assert!(human.status.success());
    let human_out = String::from_utf8_lossy(&human.stdout);
    assert!(!human_out.contains("\x1b["), "no ANSI: {human_out:?}");
    assert!(human_out.contains("dirty (landed .meta.yaml only)"), "{human_out}");

    // Preflight stops on dirty landed .meta.yaml with exit 1 + actionable JSON.
    let pre_dirty = r.run_in(&r.main, &["worktree", "preflight", "first", "--json"]);
    assert!(!pre_dirty.status.success());
    let pj1 = json_of(&pre_dirty);
    assert_eq!(pj1["readyToMerge"], false);
    assert_eq!(pj1["status"], "needs_cleanup");
    assert_eq!(pj1["submoduleMergeRequired"], true);
    assert_eq!(pj1["submoduleConflicts"], serde_json::json!(["frontend"]));
    assert_eq!(pj1["fileConflicts"], serde_json::json!([]));

    // Commit landed metadata -> preflight succeeds with status=submodule_first (exit 0).
    git_in(&r.main, &["add", "speclink/changes/prev-landed/.meta.yaml"]);
    git_in(&r.main, &["commit", "-q", "-m", "chore: record landed"]);

    let pre_sub = r.run_in(&r.main, &["worktree", "preflight", "first", "--json"]);
    assert!(
        pre_sub.status.success(),
        "submodule_first exits 0: {}",
        String::from_utf8_lossy(&pre_sub.stderr)
    );
    let pj2 = json_of(&pre_sub);
    assert_eq!(pj2["readyToMerge"], true);
    assert_eq!(pj2["status"], "submodule_first");
    assert_eq!(pj2["submoduleConflicts"], serde_json::json!(["frontend"]));
    assert_eq!(pj2["fileConflicts"], serde_json::json!([]));

    // Preflight invoked from inside a linked worktree is refused cleanly.
    let pre_in_wt = r.run_in(&wt1, &["worktree", "preflight", "first", "--json"]);
    assert!(!pre_in_wt.status.success());
    let pj_wt = json_of(&pre_in_wt);
    assert_eq!(pj_wt["readyToMerge"], false);
    assert_eq!(pj_wt["status"], "blocked");
}

#[test]
fn worktree_preflight_blocks_real_conflicts_and_supports_remote_mode_checkouts() {
    let r = TempRepo::new("remote-and-conflict");
    // Switch to remote-mode config (local git checkout only, specs on Server).
    std::fs::remove_dir_all(r.main.join("speclink")).unwrap();
    std::fs::write(
        r.main.join(".speclink.yaml"),
        "remote:\n  url: http://127.0.0.1:1/api/speclink/v1/projects/demo\n",
    )
    .unwrap();
    git_in(&r.main, &["add", "-A"]);
    git_in(&r.main, &["commit", "-q", "-m", "remote mode config"]);

    let wt_ok = r.add_worktree("clean-feat");
    std::fs::write(wt_ok.join("src/extra.rs"), "pub fn extra() {}\n").unwrap();
    git_in(&wt_ok, &["add", "src/extra.rs"]);
    git_in(&wt_ok, &["commit", "-q", "-m", "feat"]);

    let wt_bad = r.add_worktree("conflict-feat");
    std::fs::write(wt_bad.join("src/lib.rs"), "pub fn base() { let _ = 1; }\n").unwrap();
    git_in(&wt_bad, &["add", "src/lib.rs"]);
    git_in(&wt_bad, &["commit", "-q", "-m", "wt edit"]);

    std::fs::write(r.main.join("src/lib.rs"), "pub fn base() { let _ = 2; }\n").unwrap();
    git_in(&r.main, &["add", "src/lib.rs"]);
    git_in(&r.main, &["commit", "-q", "-m", "main edit"]);

    // Remote-mode local checkout never hits network and inspects clean worktree.
    let pre_ok = r.run_in(&r.main, &["worktree", "preflight", "clean-feat", "--json"]);
    assert!(
        pre_ok.status.success(),
        "remote checkout clean-feat failed: {}",
        String::from_utf8_lossy(&pre_ok.stderr)
    );
    let ok_json = json_of(&pre_ok);
    assert_eq!(ok_json["readyToMerge"], true);
    assert_eq!(ok_json["status"], "ready");

    // Conflicting worktree exits non-zero and names src/lib.rs under fileConflicts.
    let pre_bad = r.run_in(&r.main, &["worktree", "preflight", "conflict-feat", "--json"]);
    assert!(!pre_bad.status.success());
    let bad_json = json_of(&pre_bad);
    assert_eq!(bad_json["readyToMerge"], false);
    assert_eq!(bad_json["status"], "blocked");
    assert_eq!(bad_json["fileConflicts"], serde_json::json!(["src/lib.rs"]));
}
