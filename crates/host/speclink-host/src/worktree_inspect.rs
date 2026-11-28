//! Worktree, submodule (mode 160000 gitlink), landed-metadata, and Wave-1
//! overlap readiness inspection for host checkouts.
//!
//! Read-only except invoking `git` inspection commands (`worktree list`,
//! `status`, `diff`, `merge-base`, `rev-list`, `ls-files`, `ls-tree`, and
//! `merge-tree --write-tree` when supported). Never mutates git state or
//! workspace files.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde::Serialize;
use speclink_core::archive::{self, ArchiveOptions};
use speclink_core::model::{Change, ChangeMeta};
use speclink_core::plan::{self, Overlap, Plan};
use speclink_core::station::{self, OpenTicket, StationProgress, REVIEW, VERIFY};
use speclink_core::store::Store;
use speclink_core::tasks;
use speclink_core::util;
use speclink_core::workspace::Workspace;

use crate::worktree::{parse_records, WorktreeEntry, WorktreeOverlay};

const WORKTREE_BRANCH_PREFIX: &str = "speclink/";

/// Dirty state of a tracked `160000` gitlink submodule in a checkout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmoduleState {
    pub path: String,
    pub index_commit: Option<String>,
    pub head_commit: Option<String>,
    pub dirty: bool,
}

/// Uncommitted `<spec_dir>/changes/<name>/.meta.yaml` in main checkout that
/// already carries a `landed_commit` record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LandedMetaDirty {
    pub change: String,
    pub path: String,
    pub landed_commit: String,
    pub tasks_done: bool,
    pub open_tickets: Vec<String>,
    pub ready_to_archive: bool,
}

/// Cleanliness snapshot of the repository's main checkout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MainCheckoutState {
    pub branch: Option<String>,
    pub head: Option<String>,
    pub clean: bool,
    pub clean_except_landed_meta: bool,
    pub dirty_landed_meta: Vec<LandedMetaDirty>,
    pub dirty_paths: Vec<String>,
    pub dirty_submodules: Vec<SubmoduleState>,
}

/// Submodule (`160000` gitlink) commit movement on a worktree branch relative
/// to the merge target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmoduleChange {
    pub path: String,
    pub base_commit: Option<String>,
    pub branch_commit: Option<String>,
    pub target_commit: Option<String>,
    pub diverged_on_target: bool,
}

/// Overlap between two active worktrees across capabilities, file paths, or
/// gitlink submodules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeOverlap {
    pub change: String,
    pub capabilities: Vec<String>,
    pub shared_paths: Vec<String>,
    pub shared_submodules: Vec<String>,
}

/// Compact quality-station status inside a worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeStationSummary {
    pub state: String,
    pub rounds: usize,
    pub commit: Option<String>,
}

/// Actionable merge readiness classification for an active worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeReadiness {
    Ready,
    SubmoduleFirst,
    NeedsCleanup,
    Blocked,
}

impl MergeReadiness {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::SubmoduleFirst => "submodule_first",
            Self::NeedsCleanup => "needs_cleanup",
            Self::Blocked => "blocked",
        }
    }
}

/// Wave 1 change whose capability deltas overlap with sibling changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Wave1Overlap {
    pub change: String,
    pub stage: String,
    pub has_worktree: bool,
    pub overlaps: Vec<Overlap>,
    pub conflicts_with: Vec<String>,
    pub archive_after: Vec<String>,
}

/// Full inspection snapshot for a single `speclink/<change>` worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeStatusItem {
    pub change: String,
    pub branch: String,
    pub path: String,
    pub mapped: bool,
    pub clean: bool,
    pub dirty_paths: Vec<String>,
    pub dirty_submodules: Vec<SubmoduleState>,
    pub ahead: usize,
    pub behind: usize,
    pub merge_base: Option<String>,
    pub changed_paths: Vec<String>,
    pub submodules_changed: Vec<SubmoduleChange>,
    pub overlaps: Vec<Overlap>,
    pub archive_after: Vec<String>,
    pub overlapping_worktrees: Vec<WorktreeOverlap>,
    pub landed: bool,
    pub tasks_completed: usize,
    pub tasks_total: usize,
    pub review: WorktreeStationSummary,
    pub verify: WorktreeStationSummary,
    pub merge_readiness: MergeReadiness,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

/// Project-wide worktree, submodule, and Wave-1 overlap report (`worktree status`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeStatusReport {
    pub target: String,
    pub target_commit: String,
    pub in_linked_worktree: bool,
    pub main: MainCheckoutState,
    pub wave1_overlaps: Vec<Wave1Overlap>,
    pub worktrees: Vec<WorktreeStatusItem>,
}

/// Pre-merge readiness gate report for one change worktree (`worktree preflight`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreePreflightReport {
    pub change: String,
    pub target: String,
    pub target_commit: String,
    pub ready_to_merge: bool,
    pub submodule_merge_required: bool,
    pub status: MergeReadiness,
    pub main: MainCheckoutState,
    pub worktree: Option<WorktreeStatusItem>,
    pub submodule_conflicts: Vec<String>,
    pub file_conflicts: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub remediations: Vec<String>,
}

struct BranchDiff {
    merge_base: Option<String>,
    ahead: usize,
    behind: usize,
    changed_paths: Vec<String>,
    submodules_changed: Vec<SubmoduleChange>,
}

/// Inspect main checkout cleanliness, Wave-1 capability overlaps, and every
/// active `speclink/*` linked worktree.
pub fn status(
    ws: &Workspace,
    main_store: Option<&dyn Store>,
    target: Option<&str>,
    open_store: impl Fn(&Path) -> Box<dyn Store>,
) -> Result<WorktreeStatusReport> {
    let topology = discover_topology(ws, main_store, target, &open_store)?;
    Ok(topology.report)
}

/// Run the pre-merge gate for `change` against `target` (defaulting to main
/// checkout's `HEAD`). Separates solvable `160000` gitlink divergence from
/// real source/spec conflicts and distinguishes landed-only `.meta.yaml`
/// dirtiness on main checkout.
pub fn preflight(
    ws: &Workspace,
    main_store: Option<&dyn Store>,
    change: &str,
    target: Option<&str>,
    open_store: impl Fn(&Path) -> Box<dyn Store>,
) -> Result<WorktreePreflightReport> {
    let topology = discover_topology(ws, main_store, target, &open_store)?;
    let target_label = topology.report.target.clone();
    let target_commit = topology.report.target_commit.clone();
    let in_linked = topology.report.in_linked_worktree;
    let main = topology.report.main.clone();
    let worktree = topology.report.worktrees.into_iter().find(|w| w.change == change);

    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut remediations = Vec::new();
    let mut submodule_conflicts = Vec::new();
    let mut file_conflicts = Vec::new();
    let mut submodule_merge_required = false;
    let mut has_cleanup_only = false;

    if in_linked {
        blockers.push("run worktree preflight/merge from the main checkout, not inside a linked worktree".to_string());
        remediations.push(format!("cd {}", topology.main_root.display()));
    }

    append_main_blockers(&main, &mut blockers, &mut remediations, &mut has_cleanup_only);

    match &worktree {
        None => {
            blockers.push(format!("no active linked worktree found for change '{change}' (expected branch '{WORKTREE_BRANCH_PREFIX}{change}')"));
        }
        Some(wt) => {
            warnings.extend(wt.warnings.iter().cloned());
            if !wt.clean {
                has_cleanup_only = true;
                if !wt.dirty_paths.is_empty() {
                    blockers.push(format!(
                        "worktree '{}' has uncommitted file(s): {}",
                        wt.change,
                        wt.dirty_paths.join(", ")
                    ));
                }
                if !wt.dirty_submodules.is_empty() {
                    let subs: Vec<&str> = wt.dirty_submodules.iter().map(|s| s.path.as_str()).collect();
                    blockers.push(format!(
                        "worktree '{}' has dirty submodule(s): {}",
                        wt.change,
                        subs.join(", ")
                    ));
                }
                remediations.push(format!(
                    "commit or discard uncommitted changes inside {} before merging",
                    wt.path
                ));
            }

            for sub in wt.submodules_changed.iter().filter(|s| s.diverged_on_target) {
                submodule_conflicts.push(sub.path.clone());
            }

            if let Some(mt) = merge_tree_conflicts(&topology.main_root, &target_commit, &wt.branch) {
                let sub_paths = submodule_paths_across(&topology.main_root, &target_commit, &wt.branch);
                for path in mt {
                    if sub_paths.contains(&path) {
                        submodule_conflicts.push(path);
                    } else {
                        file_conflicts.push(path);
                    }
                }
            }

            submodule_conflicts.sort();
            submodule_conflicts.dedup();
            file_conflicts.sort();
            file_conflicts.dedup();

            if !submodule_conflicts.is_empty() {
                submodule_merge_required = true;
                for sub_path in &submodule_conflicts {
                    remediations.push(format!(
                        "submodule '{sub_path}' (160000 gitlink) diverged: merge '{}' inside {}/{sub_path} first, then merge '{}' in main checkout and `git add {sub_path}`",
                        wt.branch,
                        topology.main_root.display(),
                        wt.branch
                    ));
                }
            }
            if !file_conflicts.is_empty() {
                blockers.push(format!(
                    "git merge-tree detected source/spec conflict(s): {}",
                    file_conflicts.join(", ")
                ));
                remediations.push("resolve or split the conflicting files before merging".to_string());
            }
        }
    }

    let status = if !file_conflicts.is_empty()
        || worktree.is_none()
        || in_linked
        || !main.dirty_paths.is_empty()
        || !main.dirty_submodules.is_empty()
    {
        MergeReadiness::Blocked
    } else if !blockers.is_empty() && has_cleanup_only {
        MergeReadiness::NeedsCleanup
    } else if !blockers.is_empty() {
        MergeReadiness::Blocked
    } else if submodule_merge_required {
        MergeReadiness::SubmoduleFirst
    } else {
        MergeReadiness::Ready
    };

    let ready_to_merge = matches!(status, MergeReadiness::Ready | MergeReadiness::SubmoduleFirst);

    Ok(WorktreePreflightReport {
        change: change.to_string(),
        target: target_label,
        target_commit,
        ready_to_merge,
        submodule_merge_required,
        status,
        main,
        worktree,
        submodule_conflicts,
        file_conflicts,
        blockers,
        warnings,
        remediations,
    })
}

struct DiscoveredTopology {
    main_root: PathBuf,
    report: WorktreeStatusReport,
}

fn discover_topology(
    ws: &Workspace,
    main_store: Option<&dyn Store>,
    target: Option<&str>,
    open_store: &impl Fn(&Path) -> Box<dyn Store>,
) -> Result<DiscoveredTopology> {
    let (main_root, main_branch, raw_worktrees) = parse_worktree_topology(&ws.root)?;
    let in_linked_worktree = util::is_linked_worktree(&ws.root);
    let main_head = util::git(&main_root, &["rev-parse", "--verify", "HEAD^{commit}"]);
    let (target_label, target_commit) = resolve_target(&main_root, main_branch.as_deref(), target)?;

    // Resolve `main_store` against the real main checkout root even if invoked from a linked WT.
    let main_store_owned: Option<Box<dyn Store>> = if in_linked_worktree {
        main_root
            .join(&ws.spec_dir_name)
            .is_dir()
            .then(|| open_store(&main_root))
    } else {
        None
    };
    let effective_main_store: Option<&dyn Store> = match (&main_store_owned, main_store) {
        (Some(owned), _) => Some(owned.as_ref()),
        (None, Some(s)) => Some(s),
        (None, None) => None,
    };

    let main_state = inspect_main_state(
        &main_root,
        main_branch.as_deref(),
        main_head,
        &ws.spec_dir_name,
        effective_main_store,
    )?;

    let mut mapped_entries: Vec<(&WorktreeEntry, &str)> = Vec::new();
    for wt in &raw_worktrees {
        let Some(change) = wt.branch.strip_prefix(WORKTREE_BRANCH_PREFIX).filter(|s| !s.is_empty()) else {
            continue;
        };
        mapped_entries.push((wt, change));
    }

    let plan_report: Option<Plan> = effective_main_store.and_then(|store| {
        let stores: BTreeMap<String, Box<dyn Store>> = mapped_entries
            .iter()
            .filter(|(_, change)| store.change_exists(change))
            .map(|(wt, change)| ((*change).to_string(), open_store(&wt.path)))
            .collect();
        let overlay = WorktreeOverlay::new(store, stores);
        plan::compute(&overlay).ok()
    });

    let active_wt_names: BTreeSet<&str> = mapped_entries.iter().map(|(_, c)| *c).collect();
    let wave1_overlaps = collect_wave1_overlaps(plan_report.as_ref(), &active_wt_names);

    let mut branch_diffs: Vec<BranchDiff> = Vec::with_capacity(mapped_entries.len());
    for (wt, _) in &mapped_entries {
        branch_diffs.push(diff_branch_vs_target(&main_root, &target_commit, &wt.branch)?);
    }

    let mut items = Vec::with_capacity(mapped_entries.len());
    for (idx, (wt, change)) in mapped_entries.iter().enumerate() {
        let wt_store = open_store(&wt.path);
        let mapped = effective_main_store.is_some_and(|s| s.change_exists(change));
        let (dirty_paths, dirty_submodules) = inspect_worktree_dirtiness(&wt.path)?;
        let clean = dirty_paths.is_empty() && dirty_submodules.is_empty();
        let diff = &branch_diffs[idx];

        let plan_entry = plan_report
            .as_ref()
            .and_then(|p| p.changes.iter().find(|c| c.name == *change));
        let overlaps = plan_entry.map(|c| c.overlaps.clone()).unwrap_or_default();
        let archive_after = plan_entry.map(|c| c.archive_after.clone()).unwrap_or_default();

        let change_meta_raw = wt_store.read_change_meta(change).or_else(|| {
            effective_main_store.and_then(|s| s.read_change_meta(change))
        });
        let meta = ChangeMeta::from_text(change_meta_raw.as_deref()).unwrap_or_default();
        let counts = tasks::counts_for(wt_store.as_ref(), change);
        let review = station_summary(&wt.path, wt_store.as_ref(), change, &REVIEW, &meta);
        let verify = station_summary(&wt.path, wt_store.as_ref(), change, &VERIFY, &meta);
        let landed = meta.landed_commit.is_some();

        items.push((
            idx,
            WorktreeStatusItem {
                change: (*change).to_string(),
                branch: wt.branch.clone(),
                path: util::to_slash(&wt.path),
                mapped,
                clean,
                dirty_paths,
                dirty_submodules,
                ahead: diff.ahead,
                behind: diff.behind,
                merge_base: diff.merge_base.clone(),
                changed_paths: diff.changed_paths.clone(),
                submodules_changed: diff.submodules_changed.clone(),
                overlaps,
                archive_after,
                overlapping_worktrees: Vec::new(),
                landed,
                tasks_completed: counts.complete,
                tasks_total: counts.total,
                review,
                verify,
                merge_readiness: MergeReadiness::Ready,
                blockers: Vec::new(),
                warnings: Vec::new(),
            },
        ));
    }

    // Populate pairwise sibling overlaps across active worktrees, then derive readiness & warnings.
    let mut status_items: Vec<WorktreeStatusItem> = items.into_iter().map(|(_, item)| item).collect();
    let pairwise: Vec<Vec<WorktreeOverlap>> = (0..status_items.len())
        .map(|i| compute_sibling_overlaps(i, &status_items))
        .collect();
    for (item, siblings) in status_items.iter_mut().zip(pairwise) {
        item.overlapping_worktrees = siblings;
        populate_item_readiness(item, &main_state, in_linked_worktree);
    }

    Ok(DiscoveredTopology {
        main_root,
        report: WorktreeStatusReport {
            target: target_label,
            target_commit,
            in_linked_worktree,
            main: main_state,
            wave1_overlaps,
            worktrees: status_items,
        },
    })
}

fn compute_sibling_overlaps(idx: usize, items: &[WorktreeStatusItem]) -> Vec<WorktreeOverlap> {
    let own = &items[idx];
    let own_sub_set: BTreeSet<&str> = own.submodules_changed.iter().map(|s| s.path.as_str()).collect();
    let own_paths: BTreeSet<&str> = own
        .changed_paths
        .iter()
        .map(String::as_str)
        .filter(|p| !own_sub_set.contains(p))
        .collect();

    let mut out = Vec::new();
    for (j, other) in items.iter().enumerate() {
        if i_eq(idx, j) {
            continue;
        }
        let capabilities = own
            .overlaps
            .iter()
            .find(|o| o.change == other.change)
            .map(|o| o.capabilities.clone())
            .unwrap_or_default();
        let other_sub_set: BTreeSet<&str> =
            other.submodules_changed.iter().map(|s| s.path.as_str()).collect();
        let shared_submodules: Vec<String> = own_sub_set
            .intersection(&other_sub_set)
            .map(|s| (*s).to_string())
            .collect();
        let shared_paths: Vec<String> = other
            .changed_paths
            .iter()
            .map(String::as_str)
            .filter(|p| !other_sub_set.contains(p) && own_paths.contains(p))
            .map(str::to_string)
            .collect();

        if !capabilities.is_empty() || !shared_paths.is_empty() || !shared_submodules.is_empty() {
            out.push(WorktreeOverlap {
                change: other.change.clone(),
                capabilities,
                shared_paths,
                shared_submodules,
            });
        }
    }
    out
}

#[inline]
fn i_eq(a: usize, b: usize) -> bool {
    a == b
}

fn populate_item_readiness(
    item: &mut WorktreeStatusItem,
    main: &MainCheckoutState,
    in_linked_worktree: bool,
) {
    let mut has_cleanup_only = false;
    if in_linked_worktree {
        item.blockers
            .push("running from linked worktree; switch to main checkout to merge".to_string());
    }
    if !main.dirty_paths.is_empty() {
        item.blockers
            .push(format!("main checkout has dirty file(s): {}", main.dirty_paths.join(", ")));
    }
    if !main.dirty_submodules.is_empty() {
        let subs: Vec<&str> = main.dirty_submodules.iter().map(|s| s.path.as_str()).collect();
        item.blockers
            .push(format!("main checkout has dirty submodule(s): {}", subs.join(", ")));
    }
    if !main.dirty_landed_meta.is_empty() {
        has_cleanup_only = true;
        let names: Vec<&str> = main.dirty_landed_meta.iter().map(|m| m.change.as_str()).collect();
        item.blockers.push(format!(
            "main checkout has uncommitted landed metadata (.meta.yaml) for: {}",
            names.join(", ")
        ));
    }
    if !item.clean {
        has_cleanup_only = true;
        if !item.dirty_paths.is_empty() {
            item.blockers.push(format!(
                "worktree has uncommitted file(s): {}",
                item.dirty_paths.join(", ")
            ));
        }
        if !item.dirty_submodules.is_empty() {
            let subs: Vec<&str> = item.dirty_submodules.iter().map(|s| s.path.as_str()).collect();
            item.blockers.push(format!(
                "worktree has dirty submodule(s): {}",
                subs.join(", ")
            ));
        }
    }

    let has_diverged_submodule = item.submodules_changed.iter().any(|s| s.diverged_on_target);
    if has_diverged_submodule {
        let paths: Vec<&str> = item
            .submodules_changed
            .iter()
            .filter(|s| s.diverged_on_target)
            .map(|s| s.path.as_str())
            .collect();
        item.warnings.push(format!(
            "submodule gitlink(s) {} diverged on target — merge inside submodule before resolving superproject pointer",
            paths.join(", ")
        ));
    }
    if !item.overlapping_worktrees.is_empty() {
        let peers: Vec<&str> = item
            .overlapping_worktrees
            .iter()
            .map(|o| o.change.as_str())
            .collect();
        item.warnings.push(format!(
            "overlaps with active sibling worktree(s) [{}]; defer or refresh review/verify stamps after merging into main",
            peers.join(", ")
        ));
    }
    if !item.archive_after.is_empty() {
        item.warnings.push(format!(
            "archive after: {}",
            item.archive_after.join(", ")
        ));
    }
    if !item.mapped {
        item.warnings.push(
            "change directory exists in worktree branch only (not yet on target branch)"
                .to_string(),
        );
    }

    item.merge_readiness = if in_linked_worktree
        || !main.dirty_paths.is_empty()
        || !main.dirty_submodules.is_empty()
    {
        MergeReadiness::Blocked
    } else if !item.blockers.is_empty() && has_cleanup_only {
        MergeReadiness::NeedsCleanup
    } else if !item.blockers.is_empty() {
        MergeReadiness::Blocked
    } else if has_diverged_submodule {
        MergeReadiness::SubmoduleFirst
    } else {
        MergeReadiness::Ready
    };
}

fn append_main_blockers(
    main: &MainCheckoutState,
    blockers: &mut Vec<String>,
    remediations: &mut Vec<String>,
    has_cleanup_only: &mut bool,
) {
    if !main.dirty_paths.is_empty() {
        blockers.push(format!(
            "main checkout has dirty file(s): {}",
            main.dirty_paths.join(", ")
        ));
        remediations.push(
            "clean, commit, or stash uncommitted files in main checkout before merging".to_string(),
        );
    }
    if !main.dirty_submodules.is_empty() {
        let subs: Vec<&str> = main.dirty_submodules.iter().map(|s| s.path.as_str()).collect();
        blockers.push(format!(
            "main checkout has dirty submodule(s): {}",
            subs.join(", ")
        ));
        remediations.push(format!(
            "sync or clean submodule(s) {} in main checkout (`git submodule update` or clean inside submodule)",
            subs.join(", ")
        ));
    }
    if !main.dirty_landed_meta.is_empty() {
        *has_cleanup_only = true;
        let names: Vec<&str> = main.dirty_landed_meta.iter().map(|m| m.change.as_str()).collect();
        blockers.push(format!(
            "main checkout has uncommitted landed metadata (.meta.yaml) for: {}",
            names.join(", ")
        ));
        for entry in &main.dirty_landed_meta {
            if entry.ready_to_archive {
                remediations.push(format!(
                    "run `speclink archive {}` and commit, or commit `{}` cleanly before merging the next worktree",
                    entry.change, entry.path
                ));
            } else {
                remediations.push(format!(
                    "commit `{}` (`landed_commit` already recorded for `{}`) cleanly before merging the next worktree",
                    entry.path, entry.change
                ));
            }
        }
    }
}

fn collect_wave1_overlaps(
    plan_report: Option<&Plan>,
    active_wt_names: &BTreeSet<&str>,
) -> Vec<Wave1Overlap> {
    let Some(plan_report) = plan_report else {
        return Vec::new();
    };
    plan_report
        .changes
        .iter()
        .filter(|c| {
            c.wave == 1
                && (!c.overlaps.is_empty()
                    || c.requirement_overlap.iter().any(|r| r.conflict))
        })
        .map(|c| {
            let mut conflicts_with: Vec<String> = c
                .requirement_overlap
                .iter()
                .filter(|r| r.conflict)
                .map(|r| r.change.clone())
                .collect();
            conflicts_with.sort();
            conflicts_with.dedup();
            Wave1Overlap {
                change: c.name.clone(),
                stage: c.stage.as_str().to_string(),
                has_worktree: active_wt_names.contains(c.name.as_str()),
                overlaps: c.overlaps.clone(),
                conflicts_with,
                archive_after: c.archive_after.clone(),
            }
        })
        .collect()
}

fn parse_worktree_topology(root: &Path) -> Result<(PathBuf, Option<String>, Vec<WorktreeEntry>)> {
    let Some(text) = util::git(root, &["worktree", "list", "--porcelain"]) else {
        bail!("'{}' is not inside a git repository", root.display());
    };
    let mut records = parse_records(&text).into_iter();
    let Some(main_rec) = records.next() else {
        bail!("git worktree list returned no main checkout");
    };
    if main_rec.ineligible {
        bail!("main git repository is bare or unavailable for worktree merge");
    }
    let linked = records
        .filter(|r| !r.ineligible)
        .filter_map(|r| r.branch.map(|branch| WorktreeEntry { path: r.path, branch }))
        .collect();
    Ok((main_rec.path, main_rec.branch, linked))
}

fn resolve_target(
    main_root: &Path,
    main_branch: Option<&str>,
    target: Option<&str>,
) -> Result<(String, String)> {
    if let Some(rev) = target {
        if rev.starts_with('-') {
            bail!("invalid target revision '{rev}': starts with '-'");
        }
        let sha = git_trimmed(main_root, &["rev-parse", "--verify", &format!("{rev}^{{commit}}")])
            .with_context(|| format!("cannot resolve target revision '{rev}'"))?;
        return Ok((rev.to_string(), sha));
    }
    let sha = git_trimmed(main_root, &["rev-parse", "--verify", "HEAD^{commit}"])
        .context("main checkout has no commits yet")?;
    let label = main_branch
        .map(str::to_string)
        .unwrap_or_else(|| sha.chars().take(8).collect());
    Ok((label, sha))
}

fn inspect_main_state(
    main_root: &Path,
    main_branch: Option<&str>,
    main_head: Option<String>,
    spec_dir_name: &str,
    main_store: Option<&dyn Store>,
) -> Result<MainCheckoutState> {
    let submodules = stage_submodules(main_root);
    let entries = git_status_entries(main_root)?;

    let mut dirty_landed_meta = Vec::new();
    let mut dirty_paths = Vec::new();
    let mut dirty_submodules = Vec::new();

    for (xy, path) in entries {
        if let Some(idx_sha) = submodules.get(&path) {
            dirty_submodules.push(inspect_submodule_at(main_root, &path, Some(idx_sha.clone())));
            continue;
        }
        if !xy.contains('D') {
            if let Some(landed) = try_classify_landed_meta(
                main_root,
                spec_dir_name,
                &path,
                main_store,
            ) {
                dirty_landed_meta.push(landed);
                continue;
            }
        }
        dirty_paths.push(path);
    }

    dirty_paths.sort();
    dirty_paths.dedup();
    dirty_submodules.sort_by(|a, b| a.path.cmp(&b.path));
    dirty_landed_meta.sort_by(|a, b| a.change.cmp(&b.change));

    let clean = dirty_paths.is_empty() && dirty_submodules.is_empty() && dirty_landed_meta.is_empty();
    let clean_except_landed_meta =
        dirty_paths.is_empty() && dirty_submodules.is_empty() && !dirty_landed_meta.is_empty();

    Ok(MainCheckoutState {
        branch: main_branch.map(str::to_string),
        head: main_head,
        clean,
        clean_except_landed_meta,
        dirty_landed_meta,
        dirty_paths,
        dirty_submodules,
    })
}

fn try_classify_landed_meta(
    main_root: &Path,
    spec_dir_name: &str,
    rel_path: &str,
    main_store: Option<&dyn Store>,
) -> Option<LandedMetaDirty> {
    let prefix = format!("{spec_dir_name}/changes/");
    let change = rel_path.strip_prefix(&prefix)?.strip_suffix("/.meta.yaml")?;
    if change.is_empty() || change.contains('/') || change == "archive" {
        return None;
    }
    let raw = std::fs::read_to_string(main_root.join(rel_path)).ok()?;
    let meta = ChangeMeta::from_text(Some(&raw)).ok()?;
    let landed_commit = meta.landed_commit.clone()?;

    let (tasks_done, open_tickets, ready_to_archive) = match main_store {
        Some(store) => {
            let counts = tasks::counts_for(store, change);
            let open_tickets: Vec<String> = [&REVIEW, &VERIFY]
                .into_iter()
                .filter(|st| store.artifact_exists(change, st.doc))
                .map(|st| st.noun.to_string())
                .collect();
            let change_obj: Option<Change> = store.find_change(change);
            let skip = change_obj
                .as_ref()
                .and_then(|c| archive::skip_reason(store, c, &ArchiveOptions::default()));
            let any_stale = [&REVIEW, &VERIFY].into_iter().any(|st| {
                let status = station::status(st, &meta, None);
                if matches!(status.progress, StationProgress::Stamped) {
                    let reader = |p: &str| util::read_scope_bytes(main_root, p);
                    station::stale_reason(meta.anchors(st), &counts, Some(&reader)).is_some()
                } else {
                    false
                }
            });
            let tasks_done = !matches!(skip, Some(archive::SkipReason::TasksIncomplete { .. }))
                && counts.remaining == 0;
            let ready_to_archive =
                change_obj.is_some() && open_tickets.is_empty() && !any_stale && skip.is_none();
            (tasks_done, open_tickets, ready_to_archive)
        }
        None => (false, Vec::new(), false),
    };

    Some(LandedMetaDirty {
        change: change.to_string(),
        path: rel_path.to_string(),
        landed_commit,
        tasks_done,
        open_tickets,
        ready_to_archive,
    })
}

fn inspect_worktree_dirtiness(root: &Path) -> Result<(Vec<String>, Vec<SubmoduleState>)> {
    let submodules = stage_submodules(root);
    let entries = git_status_entries(root)?;
    let mut dirty_paths = Vec::new();
    let mut dirty_submodules = Vec::new();
    for (_, path) in entries {
        if let Some(idx_sha) = submodules.get(&path) {
            dirty_submodules.push(inspect_submodule_at(root, &path, Some(idx_sha.clone())));
        } else {
            dirty_paths.push(path);
        }
    }
    dirty_paths.sort();
    dirty_paths.dedup();
    dirty_submodules.sort_by(|a, b| a.path.cmp(&b.path));
    Ok((dirty_paths, dirty_submodules))
}

fn inspect_submodule_at(
    root: &Path,
    rel_path: &str,
    index_commit: Option<String>,
) -> SubmoduleState {
    let sub_dir = root.join(rel_path);
    if !sub_dir.exists() {
        return SubmoduleState {
            path: rel_path.to_string(),
            index_commit,
            head_commit: None,
            dirty: false,
        };
    }
    let head_commit = util::git(&sub_dir, &["rev-parse", "--verify", "HEAD"]);
    let dirty = util::git_raw(
        &sub_dir,
        &["-c", "core.quotepath=false", "status", "--porcelain", "-uall", "-z"],
    )
    .is_some_and(|raw| !raw.is_empty());
    SubmoduleState {
        path: rel_path.to_string(),
        index_commit,
        head_commit,
        dirty,
    }
}

fn stage_submodules(root: &Path) -> BTreeMap<String, String> {
    let Some(raw) = git_stdout_bytes(
        root,
        &["-c", "core.quotepath=false", "ls-files", "--stage", "-z"],
    ) else {
        return BTreeMap::new();
    };
    let mut out = BTreeMap::new();
    for entry in raw.split(|&b| b == 0).filter(|e| !e.is_empty()) {
        let Ok(line) = std::str::from_utf8(entry) else {
            continue;
        };
        let Some((meta, path)) = line.split_once('\t') else {
            continue;
        };
        let mut parts = meta.split_whitespace();
        if parts.next() == Some("160000") {
            if let Some(sha) = parts.next() {
                out.insert(path.to_string(), sha.to_string());
            }
        }
    }
    out
}

fn submodule_paths_across(root: &Path, target_commit: &str, branch: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for rev in [target_commit, branch] {
        let Some(raw) = git_stdout_bytes(
            root,
            &["-c", "core.quotepath=false", "ls-tree", "-r", "-z", rev],
        ) else {
            continue;
        };
        for entry in raw.split(|&b| b == 0).filter(|e| !e.is_empty()) {
            let Ok(line) = std::str::from_utf8(entry) else {
                continue;
            };
            let Some((meta, path)) = line.split_once('\t') else {
                continue;
            };
            if meta.starts_with("160000 ") {
                out.insert(path.to_string());
            }
        }
    }
    out
}

fn git_status_entries(root: &Path) -> Result<Vec<(String, String)>> {
    let raw = git_stdout_bytes(
        root,
        &["-c", "core.quotepath=false", "status", "--porcelain", "-uall", "-z"],
    )
    .with_context(|| format!("cannot inspect git status in '{}'", root.display()))?;
    let fields: Vec<&[u8]> = raw.split(|&b| b == 0).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < fields.len() {
        let entry = fields[i];
        i += 1;
        if entry.len() < 4 || entry[2] != b' ' {
            continue;
        }
        let xy = String::from_utf8_lossy(&entry[..2]).to_string();
        let path = String::from_utf8_lossy(&entry[3..]).to_string();
        if xy.bytes().any(|b| b == b'R' || b == b'C') {
            // Rename/copy porcelain `-z` emits the source path as the next NUL token.
            i += 1;
        }
        out.push((xy, path));
    }
    Ok(out)
}

fn diff_branch_vs_target(
    main_root: &Path,
    target_commit: &str,
    branch_name: &str,
) -> Result<BranchDiff> {
    let branch_ref = format!("refs/heads/{branch_name}");
    let branch_commit = git_trimmed(main_root, &["rev-parse", "--verify", &format!("{branch_ref}^{{commit}}")])
        .with_context(|| format!("cannot resolve branch '{branch_name}'"))?;
    let merge_base = util::git(main_root, &["merge-base", target_commit, &branch_commit]);

    let (ahead, behind) = match util::git(
        main_root,
        &["rev-list", "--left-right", "--count", &format!("{target_commit}...{branch_commit}")],
    ) {
        Some(counts) => {
            let mut parts = counts.split_whitespace();
            let b = parts.next().and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
            let a = parts.next().and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
            (a, b)
        }
        None => (0, 0),
    };

    let Some(ref base) = merge_base else {
        return Ok(BranchDiff {
            merge_base: None,
            ahead,
            behind,
            changed_paths: Vec::new(),
            submodules_changed: Vec::new(),
        });
    };

    let changed_paths = git_stdout_bytes(
        main_root,
        &["-c", "core.quotepath=false", "diff", "--name-only", "-z", base, &branch_commit],
    )
    .map(|bytes| {
        let mut v: Vec<String> = bytes
            .split(|&b| b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| String::from_utf8_lossy(s).to_string())
            .collect();
        v.sort();
        v.dedup();
        v
    })
    .unwrap_or_default();

    let mut submodules_changed = Vec::new();
    if let Some(raw) = git_stdout_bytes(
        main_root,
        &["-c", "core.quotepath=false", "diff", "--raw", "--no-renames", "-z", base, &branch_commit],
    ) {
        let tokens: Vec<&[u8]> = raw.split(|&b| b == 0).collect();
        let mut i = 0;
        while i + 1 < tokens.len() {
            let hdr = tokens[i];
            let path_bytes = tokens[i + 1];
            i += 2;
            if hdr.is_empty() || path_bytes.is_empty() {
                continue;
            }
            let Ok(hdr_str) = std::str::from_utf8(hdr) else {
                continue;
            };
            let mut cols = hdr_str.trim_start_matches(':').split_whitespace();
            let (Some(old_mode), Some(new_mode)) = (cols.next(), cols.next()) else {
                continue;
            };
            if old_mode == "160000" || new_mode == "160000" {
                let path = String::from_utf8_lossy(path_bytes).to_string();
                let base_oid = util::gitlink_commit_at(main_root, base, &path);
                let branch_oid = util::gitlink_commit_at(main_root, &branch_commit, &path);
                let target_oid = util::gitlink_commit_at(main_root, target_commit, &path);
                let diverged_on_target =
                    target_oid != base_oid && branch_oid != target_oid;
                submodules_changed.push(SubmoduleChange {
                    path,
                    base_commit: base_oid,
                    branch_commit: branch_oid,
                    target_commit: target_oid,
                    diverged_on_target,
                });
            }
        }
    }
    submodules_changed.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(BranchDiff {
        merge_base,
        ahead,
        behind,
        changed_paths,
        submodules_changed,
    })
}

/// Simulate merge via `git merge-tree --write-tree --name-only --no-messages -z`.
/// Returns `None` when Git is older than 2.38 or `merge-tree` cannot run cleanly.
fn merge_tree_conflicts(root: &Path, target_commit: &str, branch: &str) -> Option<Vec<String>> {
    let branch_ref = format!("refs/heads/{branch}");
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotepath=false",
            "merge-tree",
            "--write-tree",
            "--name-only",
            "--no-messages",
            "-z",
            target_commit,
            &branch_ref,
        ])
        .output()
        .ok()?;
    match out.status.code() {
        Some(0) => Some(Vec::new()),
        Some(1) => {
            // Format: `<tree_oid>\0<path1>\0...<pathN>\0\0` (when `--no-messages` is set).
            let mut fields = out.stdout.split(|&b| b == 0);
            let _tree_oid = fields.next()?;
            let mut conflicts = Vec::new();
            for field in fields {
                if field.is_empty() {
                    break;
                }
                conflicts.push(String::from_utf8_lossy(field).to_string());
            }
            Some(conflicts)
        }
        _ => None,
    }
}

fn station_summary(
    wt_root: &Path,
    store: &dyn Store,
    change: &str,
    st: &'static station::Station,
    meta: &ChangeMeta,
) -> WorktreeStationSummary {
    let ticket_exists = store.artifact_exists(change, st.doc);
    let parsed_rounds = ticket_exists
        .then(|| station::show(st, store, change).ok().map(|t| t.rounds.len()))
        .flatten();
    let open_info = ticket_exists.then(|| OpenTicket {
        rounds: parsed_rounds.and_then(|n| u64::try_from(n).ok()),
    });
    let status = station::status(st, meta, open_info);

    let counts = tasks::counts_for(store, change);
    let state = match status.progress {
        StationProgress::None => "none",
        StationProgress::InProgress => "open",
        StationProgress::Stamped => {
            let reader = |p: &str| util::read_scope_bytes(wt_root, p);
            if station::stale_reason(meta.anchors(st), &counts, Some(&reader)).is_some() {
                "stale"
            } else {
                "stamped"
            }
        }
    };

    let commit = match st.id {
        station::StationId::Review => meta.reviewed_commit.clone(),
        station::StationId::Verify => meta.verified_commit.clone(),
    };
    let rounds = status
        .rounds
        .and_then(|r| usize::try_from(r).ok())
        .or(parsed_rounds)
        .unwrap_or(0);

    WorktreeStationSummary {
        state: state.to_string(),
        rounds,
        commit,
    }
}

fn git_trimmed(root: &Path, args: &[&str]) -> Result<String> {
    util::git(root, args).with_context(|| format!("git {:?} failed in '{}'", args, root.display()))
}

fn git_stdout_bytes(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("git").arg("-C").arg(root).args(args).output().ok()?;
    out.status.success().then_some(out.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use speclink_fs::FsStore;
    use std::process::Command;

    struct RepoFixture {
        root: PathBuf,
        sub_upstream: Option<PathBuf>,
        worktrees: Vec<PathBuf>,
    }

    impl RepoFixture {
        fn new(tag: &str) -> Self {
            static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let id = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let base = std::env::temp_dir().join(format!(
                "speclink-host-wt-inspect-{tag}-{}-{id}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&base);
            let root = base.join("main");
            std::fs::create_dir_all(&root).unwrap();
            git_in(&root, &["init", "-q", "-b", "main"]);
            git_in(&root, &["config", "user.name", "Tester"]);
            git_in(&root, &["config", "user.email", "tester@example.com"]);
            std::fs::write(root.join("speclink.marker"), "").ok();
            std::fs::create_dir_all(root.join("speclink/specs/auth")).unwrap();
            std::fs::write(
                root.join("speclink/specs/auth/spec.md"),
                "# auth\n\n## Purpose\nAuth\n\n## Requirements\n\n### Requirement: Login\nShall login.\n",
            )
            .unwrap();
            std::fs::write(
                root.join("speclink/config.yaml"),
                "schema: spec-driven\nworktree: true\n",
            )
            .unwrap();
            std::fs::create_dir_all(root.join("src")).unwrap();
            std::fs::write(root.join("src/lib.rs"), "pub fn base() {}\n").unwrap();
            git_in(&root, &["add", "-A"]);
            git_in(&root, &["commit", "-q", "-m", "init"]);
            Self { root, sub_upstream: None, worktrees: Vec::new() }
        }

        fn add_submodule(&mut self, name: &str) -> PathBuf {
            let sub_up = self.root.parent().unwrap().join(format!("{name}-upstream"));
            std::fs::create_dir_all(&sub_up).unwrap();
            git_in(&sub_up, &["init", "-q", "-b", "main"]);
            git_in(&sub_up, &["config", "user.name", "Tester"]);
            git_in(&sub_up, &["config", "user.email", "tester@example.com"]);
            std::fs::write(sub_up.join("app.txt"), "v1\n").unwrap();
            git_in(&sub_up, &["add", "-A"]);
            git_in(&sub_up, &["commit", "-q", "-m", "sub init"]);
            git_in(
                &self.root,
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
            git_in(&self.root, &["commit", "-q", "-m", "add sub"]);
            self.sub_upstream = Some(sub_up.clone());
            sub_up
        }

        fn put_change(&self, dir: &Path, name: &str, cap: &str, tasks_md: &str) {
            let cdir = dir.join("speclink/changes").join(name);
            std::fs::create_dir_all(cdir.join("specs").join(cap)).unwrap();
            std::fs::write(cdir.join(".meta.yaml"), "schema: spec-driven\ncreated: 2026-11-28\n")
                .unwrap();
            std::fs::write(cdir.join("proposal.md"), "## Why\nFix.\n\n## What Changes\n- x\n")
                .unwrap();
            std::fs::write(
                cdir.join("specs").join(cap).join("spec.md"),
                "## ADDED Requirements\n\n### Requirement: Extra\nShall work.\n",
            )
            .unwrap();
            std::fs::write(cdir.join("tasks.md"), tasks_md).unwrap();
        }

        fn add_worktree(&mut self, change: &str) -> PathBuf {
            let wt = self.root.parent().unwrap().join(format!("wt-{change}"));
            let branch = format!("speclink/{change}");
            git_in(
                &self.root,
                &["worktree", "add", "-q", "-b", &branch, wt.to_str().unwrap()],
            );
            // Initialize submodule inside linked worktree when present.
            if self.sub_upstream.is_some() {
                git_in(
                    &wt,
                    &["-c", "protocol.file.allow=always", "submodule", "update", "--init", "-q"],
                );
            }
            self.worktrees.push(wt.clone());
            wt
        }

        fn ws(&self) -> Workspace {
            Workspace { root: self.root.clone(), spec_dir_name: "speclink".to_string() }
        }
    }

    impl Drop for RepoFixture {
        fn drop(&mut self) {
            if let Some(base) = self.root.parent() {
                let _ = std::fs::remove_dir_all(base);
            }
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
            "git {args:?} failed in {}:\n{}",
            dir.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).to_string()
    }

    #[test]
    fn status_surfaces_wave1_overlaps_and_submodule_divergence_and_landed_meta() {
        let mut fx = RepoFixture::new("wave1-sub");
        fx.add_submodule("frontend");
        fx.put_change(&fx.root, "first", "auth", "- [x] 1.1 done\n");
        fx.put_change(&fx.root, "second", "auth", "- [x] 1.1 done\n");
        fx.put_change(&fx.root, "landed-prev", "other", "- [x] 1.1 done\n");
        git_in(&fx.root, &["add", "-A"]);
        git_in(&fx.root, &["commit", "-q", "-m", "seed changes"]);

        let wt1 = fx.add_worktree("first");
        let wt2 = fx.add_worktree("second");

        // Advance submodule in wt1, merge wt1's submodule into main so main diverges from wt2.
        std::fs::write(wt1.join("frontend/app.txt"), "v1\nwt1\n").unwrap();
        git_in(&wt1.join("frontend"), &["add", "app.txt"]);
        git_in(&wt1.join("frontend"), &["commit", "-q", "-m", "wt1 sub"]);
        git_in(&wt1, &["add", "frontend"]);
        git_in(&wt1, &["commit", "-q", "-m", "wt1 gitlink"]);

        // Advance main's submodule independently (simulating an earlier merged branch).
        std::fs::write(fx.root.join("frontend/app.txt"), "main\nv1\n").unwrap();
        git_in(&fx.root.join("frontend"), &["add", "app.txt"]);
        git_in(&fx.root.join("frontend"), &["commit", "-q", "-m", "main sub"]);
        git_in(&fx.root, &["add", "frontend"]);
        git_in(&fx.root, &["commit", "-q", "-m", "main advance frontend"]);

        // wt2 also edits frontend & leaves review.md untracked.
        std::fs::write(wt2.join("frontend/app.txt"), "v1\nwt2\n").unwrap();
        git_in(&wt2.join("frontend"), &["add", "app.txt"]);
        git_in(&wt2.join("frontend"), &["commit", "-q", "-m", "wt2 sub"]);
        git_in(&wt2, &["add", "frontend"]);
        git_in(&wt2, &["commit", "-q", "-m", "wt2 gitlink"]);
        std::fs::write(wt2.join("speclink/changes/second/review.md"), "## Round 1\n").unwrap();

        // Simulate landed-only dirty .meta.yaml in main checkout.
        let main_head = git_in(&fx.root, &["rev-parse", "HEAD"]).trim().to_string();
        std::fs::write(
            fx.root.join("speclink/changes/landed-prev/.meta.yaml"),
            format!("schema: spec-driven\ncreated: 2026-11-28\nlanded_commit: {main_head}\n"),
        )
        .unwrap();

        let ws = fx.ws();
        let main_store = FsStore::new(&fx.root, "speclink");
        let report = status(&ws, Some(&main_store), None, |p| {
            Box::new(FsStore::new(p, "speclink"))
        })
        .expect("status succeeds");

        assert!(!report.main.clean);
        assert!(report.main.clean_except_landed_meta);
        assert_eq!(report.main.dirty_landed_meta.len(), 1);
        assert_eq!(report.main.dirty_landed_meta[0].change, "landed-prev");
        assert!(report.wave1_overlaps.iter().any(|w| w.change == "first"));
        assert!(report.wave1_overlaps.iter().any(|w| w.change == "second"));

        let item1 = report.worktrees.iter().find(|w| w.change == "first").unwrap();
        assert!(item1.clean);
        assert_eq!(item1.submodules_changed[0].path, "frontend");
        assert!(item1.submodules_changed[0].diverged_on_target);
        assert!(!item1.overlapping_worktrees.is_empty());
        assert_eq!(item1.overlapping_worktrees[0].shared_submodules, vec!["frontend"]);
        assert_eq!(item1.merge_readiness, MergeReadiness::NeedsCleanup);

        let item2 = report.worktrees.iter().find(|w| w.change == "second").unwrap();
        assert!(!item2.clean);
        assert_eq!(item2.review.state, "open");

        let pre = preflight(&ws, Some(&main_store), "first", None, |p| {
            Box::new(FsStore::new(p, "speclink"))
        })
        .expect("preflight runs");
        assert!(!pre.ready_to_merge);
        assert_eq!(pre.status, MergeReadiness::NeedsCleanup);
        assert!(pre.submodule_merge_required);
        assert_eq!(pre.submodule_conflicts, vec!["frontend"]);
        assert!(pre.file_conflicts.is_empty());

        // Commit landed-prev .meta.yaml -> main clean -> first becomes SubmoduleFirst (ready_to_merge = true).
        git_in(&fx.root, &["add", "speclink/changes/landed-prev/.meta.yaml"]);
        git_in(&fx.root, &["commit", "-q", "-m", "chore: record landed"]);

        let pre_after = preflight(&ws, Some(&main_store), "first", None, |p| {
            Box::new(FsStore::new(p, "speclink"))
        })
        .expect("preflight after cleanup");
        assert!(pre_after.ready_to_merge);
        assert_eq!(pre_after.status, MergeReadiness::SubmoduleFirst);
        assert!(pre_after.submodule_merge_required);
        assert_eq!(pre_after.submodule_conflicts, vec!["frontend"]);
        assert!(pre_after.file_conflicts.is_empty());
    }

    #[test]
    fn preflight_blocks_real_file_conflicts_distinct_from_submodule_conflicts() {
        let mut fx = RepoFixture::new("file-conflict");
        fx.put_change(&fx.root, "alpha", "auth", "- [x] 1.1 done\n");
        git_in(&fx.root, &["add", "-A"]);
        git_in(&fx.root, &["commit", "-q", "-m", "seed alpha"]);

        let wt = fx.add_worktree("alpha");
        std::fs::write(wt.join("src/lib.rs"), "pub fn base() { let _ = 1; }\n").unwrap();
        git_in(&wt, &["add", "src/lib.rs"]);
        git_in(&wt, &["commit", "-q", "-m", "wt change"]);

        std::fs::write(fx.root.join("src/lib.rs"), "pub fn base() { let _ = 2; }\n").unwrap();
        git_in(&fx.root, &["add", "src/lib.rs"]);
        git_in(&fx.root, &["commit", "-q", "-m", "main conflict"]);

        let ws = fx.ws();
        let main_store = FsStore::new(&fx.root, "speclink");
        let pre = preflight(&ws, Some(&main_store), "alpha", None, |p| {
            Box::new(FsStore::new(p, "speclink"))
        })
        .expect("preflight");
        assert!(!pre.ready_to_merge);
        assert_eq!(pre.status, MergeReadiness::Blocked);
        assert_eq!(pre.file_conflicts, vec!["src/lib.rs"]);
        assert!(pre.submodule_conflicts.is_empty());
    }
}
