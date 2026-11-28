//! Local checkout worktree & submodule (mode 160000 gitlink) inspection verbs:
//! `speclink worktree status` and `speclink worktree preflight`.
//!
//! Operates purely on local git worktree topology + optional local `speclink/`
//! overlay (`FsStore`) so both fs-mode and remote-mode checkouts can inspect
//! branch divergence, submodule conflicts, dirty landed `.meta.yaml`, and Wave-1
//! capability overlaps without touching remote state or raw untyped JSON.

use anyhow::{bail, Result};
use clap::{Args, Subcommand};
use speclink_core as core;
use speclink_host::worktree::{
    preflight, status, MergeReadiness, WorktreePreflightReport, WorktreeStatusItem,
    WorktreeStatusReport,
};

use crate::color;
use crate::common::print_json;

#[derive(Args)]
pub(crate) struct WorktreeArgs {
    #[command(subcommand)]
    command: WorktreeCommands,
}

#[derive(Subcommand)]
enum WorktreeCommands {
    /// Show main checkout cleanliness, Wave-1 overlaps, and active speclink/* worktrees
    Status {
        /// Target branch or commit to compare worktrees against (default: main HEAD)
        #[arg(long)]
        target: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Run a pre-merge readiness check for one active change worktree
    Preflight {
        /// Change name (for branch speclink/<change>)
        change: String,
        /// Target branch or commit to merge into (default: main HEAD)
        #[arg(long)]
        target: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn cmd_worktree(args: WorktreeArgs) -> Result<()> {
    let ws = resolve_checkout_workspace()?;
    let fs_store = local_fs_store(&ws)?;
    let store_ref = fs_store.as_ref().map(|s| s as &dyn core::store::Store);
    let open_store = |root: &std::path::Path| -> Box<dyn core::store::Store> {
        Box::new(speclink_fs::FsStore::new(root, &ws.spec_dir_name))
    };

    match args.command {
        WorktreeCommands::Status { target, json } => {
            let report = status(&ws, store_ref, target.as_deref(), open_store)?;
            if json {
                print_json(&report)?;
            } else {
                print_status_human(&report);
            }
            Ok(())
        }
        WorktreeCommands::Preflight { change, target, json } => {
            let report = preflight(&ws, store_ref, &change, target.as_deref(), open_store)?;
            if json {
                print_json(&report)?;
            } else {
                print_preflight_human(&report);
            }
            if !report.ready_to_merge {
                let summary = if report.blockers.is_empty() {
                    format!("worktree preflight failed for '{change}'")
                } else {
                    report.blockers.join("; ")
                };
                bail!("{summary}");
            }
            Ok(())
        }
    }
}

fn resolve_checkout_workspace() -> Result<core::workspace::Workspace> {
    if let Some(ws) = core::workspace::Workspace::discover_cwd()? {
        return Ok(ws);
    }
    let cwd = std::env::current_dir()?;
    if let Some(git_root) = core::util::git(&cwd, &["rev-parse", "--show-toplevel"]) {
        return Ok(core::workspace::Workspace {
            root: std::path::PathBuf::from(git_root),
            spec_dir_name: core::workspace::DEFAULT_SPEC_DIR.to_string(),
        });
    }
    bail!("Not in a Speclink or Git workspace.")
}

fn local_fs_store(ws: &core::workspace::Workspace) -> Result<Option<speclink_fs::FsStore>> {
    if !ws.spec_dir().is_dir() {
        return Ok(None);
    }
    if ws.app_config().is_file()
        && matches!(
            speclink_host::context::resolve_store_mode(ws)?.mode,
            core::workspace::StoreMode::Remote(_)
        )
    {
        return Ok(None);
    }
    Ok(Some(speclink_fs::FsStore::new(&ws.root, &ws.spec_dir_name)))
}

fn print_status_human(r: &WorktreeStatusReport) {
    let p = color::paint();
    let target_short: String = r.target_commit.chars().take(8).collect();
    println!(
        "{} Target: {} ({})",
        p.cyan("Worktrees"),
        r.target,
        p.dim(&target_short)
    );
    if r.in_linked_worktree {
        println!(
            "  {} running inside a linked worktree (switch to main checkout to merge)",
            p.yellow("note:")
        );
    }
    print_main_state(&r.main);

    if !r.wave1_overlaps.is_empty() {
        println!("\n{}", p.bold("Wave 1 capability overlaps:"));
        for w in &r.wave1_overlaps {
            let wt_tag = if w.has_worktree { " [worktree]" } else { "" };
            let peers: Vec<String> = w
                .overlaps
                .iter()
                .map(|o| format!("{} ({})", o.change, o.capabilities.join(", ")))
                .collect();
            let mut extra = String::new();
            if !w.archive_after.is_empty() {
                extra.push_str(&format!(" — archive after: {}", w.archive_after.join(", ")));
            }
            if !w.conflicts_with.is_empty() {
                extra.push_str(&format!(" — conflicts with: {}", w.conflicts_with.join(", ")));
            }
            println!(
                "  {} {} [{}]{wt_tag} — overlaps: {}{extra}",
                p.yellow("•"),
                w.change,
                w.stage,
                peers.join("; ")
            );
        }
    }

    if r.worktrees.is_empty() {
        println!("\n{}", p.dim("No active speclink/* worktrees."));
        return;
    }

    println!("\n{}", p.bold("Active worktrees:"));
    for item in &r.worktrees {
        print_worktree_item(item);
    }
}

fn print_preflight_human(r: &WorktreePreflightReport) {
    let p = color::paint();
    let badge = match r.status {
        MergeReadiness::Ready => p.green("READY"),
        MergeReadiness::SubmoduleFirst => p.yellow("SUBMODULE_FIRST"),
        MergeReadiness::NeedsCleanup => p.yellow("NEEDS_CLEANUP"),
        MergeReadiness::Blocked => p.red("BLOCKED"),
    };
    let target_short: String = r.target_commit.chars().take(8).collect();
    println!(
        "Preflight {}: {} (target {} {})",
        r.change,
        badge,
        r.target,
        p.dim(&target_short)
    );
    print_main_state(&r.main);
    if let Some(wt) = &r.worktree {
        print_worktree_item(wt);
    }
    if !r.submodule_conflicts.is_empty() {
        println!(
            "  {} submodule gitlink(s) to merge first: {}",
            p.yellow("!"),
            r.submodule_conflicts.join(", ")
        );
    }
    if !r.file_conflicts.is_empty() {
        println!(
            "  {} merge-tree file conflict(s): {}",
            p.red("✗"),
            r.file_conflicts.join(", ")
        );
    }
    for blocker in &r.blockers {
        println!("  {} {blocker}", p.red("blocker:"));
    }
    for warning in &r.warnings {
        println!("  {} {warning}", p.yellow("warning:"));
    }
    for step in &r.remediations {
        println!("  {} {step}", p.cyan("next:"));
    }
}

fn print_main_state(m: &speclink_host::worktree::MainCheckoutState) {
    let p = color::paint();
    let branch = m.branch.as_deref().unwrap_or("(detached HEAD)");
    let state = if m.clean {
        p.green("clean")
    } else if m.clean_except_landed_meta {
        p.yellow("dirty (landed .meta.yaml only)")
    } else {
        p.red("dirty")
    };
    println!("  Main ({branch}): {state}");
    for landed in &m.dirty_landed_meta {
        let arch = if landed.ready_to_archive { " [ready to archive]" } else { "" };
        println!("    - landed metadata: {} ({}){arch}", landed.path, landed.change);
    }
    for sub in &m.dirty_submodules {
        println!("    - dirty submodule: {}", sub.path);
    }
    for path in &m.dirty_paths {
        println!("    - dirty path: {path}");
    }
}

fn print_worktree_item(item: &WorktreeStatusItem) {
    let p = color::paint();
    let readiness = match item.merge_readiness {
        MergeReadiness::Ready => p.green(item.merge_readiness.as_str()),
        MergeReadiness::SubmoduleFirst | MergeReadiness::NeedsCleanup => {
            p.yellow(item.merge_readiness.as_str())
        }
        MergeReadiness::Blocked => p.red(item.merge_readiness.as_str()),
    };
    let clean_label = if item.clean { "clean" } else { "dirty" };
    println!(
        "  {} {} ({}) [{}] +{}/-{} tasks {}/{} review:{} verify:{} — {}",
        p.cyan("•"),
        item.change,
        item.branch,
        clean_label,
        item.ahead,
        item.behind,
        item.tasks_completed,
        item.tasks_total,
        item.review.state,
        item.verify.state,
        readiness
    );
    println!("    path: {}", item.path);
    if !item.submodules_changed.is_empty() {
        let subs: Vec<String> = item
            .submodules_changed
            .iter()
            .map(|s| {
                if s.diverged_on_target {
                    format!("{} (diverged)", s.path)
                } else {
                    s.path.clone()
                }
            })
            .collect();
        println!("    submodules: {}", subs.join(", "));
    }
    for sib in &item.overlapping_worktrees {
        let mut parts = Vec::new();
        if !sib.capabilities.is_empty() {
            parts.push(format!("caps: {}", sib.capabilities.join(", ")));
        }
        if !sib.shared_submodules.is_empty() {
            parts.push(format!("submodules: {}", sib.shared_submodules.join(", ")));
        }
        if !sib.shared_paths.is_empty() {
            parts.push(format!("paths: {}", sib.shared_paths.join(", ")));
        }
        println!("    overlaps with {}: {}", sib.change, parts.join("; "));
    }
}
