---
name: speclink-worktree-merge
description: "Use when a worktree change is committed and ready to land — merges the branch back into the main branch, then cleans the worktree up."
license: MIT
compatibility: Requires speclink CLI.
metadata:
  author: speclink
  version: "v1.67.0"
  generatedBy: "Speclink"
---

**Conversation language**: write to the user in the language they use with you — progress updates, questions, and summaries. This skill is written in English, and its example messages show structure, not language: translate their prose, and keep commands, file paths, code, and fixed tokens (severity labels, status words) as they are. When a step sets the language of its own output — a document, a report, or the way it presents results and asks about them (for example through `locale`) — that step decides for that output. When you edit a document that already exists and no step sets its language, keep the language it is written in. When you run in a forked context that cannot see the conversation, use the language of the request you were given.

Merge a finished Speclink worktree branch back into the main branch, then clean up.

This is the wrap-up half of `$speclink-apply-with-worktree`. That skill stops right after committing inside the worktree; this one takes it from there. It is **human-triggered**: merging is a decision, and a conflict is the user's call.

**Input**: Optionally specify a change name (e.g., `$speclink-worktree-merge add-auth`). If omitted, run `git worktree list --porcelain` and offer the `speclink/*` branches found. If more than one is a candidate you MUST ask which one and wait for the answer — never guess.

**Prerequisites**: This skill requires `git` and the `speclink` CLI. Run `git --version`. If git is not available, report it and STOP. Every step below is driven from the **main checkout**; the steps that act on the worktree or its submodules — status checks, submodule fetches/merges, and the rebase — reach them with `-C <path>` rather than moving you there.

**Steps**

1. **Identify the worktree**

   ```bash
   git worktree list --porcelain
   ```

   Find the entry whose branch is `speclink/<change-name>`. If there is none, STOP and tell the user no worktree exists for that change — nothing to merge.

   Announce: "Merging worktree for change: <name>" plus the worktree path and branch.

2. **Preflight — all conditions must hold**

   Check which branch the main checkout is on:

   ```bash
   git -C <main-checkout> branch --show-current
   ```

   - **A `speclink/*` branch, or empty output (detached HEAD)** — STOP. The main checkout is parked somewhere a merge must never land; tell the user to switch it back to the main branch first. Do **NOT** switch branches on their behalf.
   - **Anything else** — that branch is the merge target. Announce it (it appears again in step 3 and in the success output), so a wrong destination is visible BEFORE the merge, not after.

   Run the worktree preflight inspection and check both working trees plus any active submodules:

   ```bash
   speclink worktree preflight "<change-name>" --target "<target-branch>" --json
   git -C <main-checkout> status --porcelain
   git -C <worktree-path> status --porcelain
   git -C <main-checkout> submodule status
   ```

   Note: `speclink worktree preflight` exits non-zero when `status` is `needs_cleanup` or `blocked` while still printing JSON — read its `status`, `main`, `worktree`, `submoduleConflicts`, `fileConflicts`, and `blockers` fields.

   - **Worktree not fully committed** (any uncommitted change in `<worktree-path>` or inside any submodule it contains, shown as `dirtySubmodules` or `git -C <worktree-path>/<submodule> status --porcelain`) — STOP. List the dirty files (including dirty submodule paths) and tell the user the change's work must be committed inside the worktree (and inside the submodule first, on branch `speclink/<change-name>`) before merging (`$speclink-apply-with-worktree` does this at its wrap-up). Do **NOT** commit on their behalf.
   - **Main tree dirty only from already-landed change metadata** (`main.cleanExceptLandedMeta` is true in preflight, meaning every dirty path in `<main-checkout>` is a `speclink/changes/<other>/.meta.yaml` carrying a `landed_commit` from an earlier merge) — do not deadlock on an earlier landing record:
     - If an entry in `main.dirtyLandedMeta` reports `readyToArchive: true` and the user wants to finish that change first, run `$speclink-archive <other>` and commit its archive changes; otherwise stage each listed landed metadata file individually (`git -C <main-checkout> add "<path>"`) and commit it with a message matching the repo's commit convention (without `--no-verify`, never sweeping unrelated files), then continue once the main tree is clean.
   - **Main tree not clean** (any other uncommitted change in `<main-checkout>` or inside its submodules `main.dirtySubmodules`) — STOP. List the dirty files and tell the user to commit or stash them first. Do **NOT** stash on their behalf. Do **NOT** commit their unrelated work for them.
   - **Submodule target branch detached when a submodule merge is needed** — for each path in `submoduleConflicts` (or any `160000` submodule gitlink modified on `speclink/<change-name>` whose target submodule needs merging), verify `git -C <main-checkout>/<submodule> branch --show-current` is non-empty. If detached HEAD, stop and tell the user to check out the submodule's target branch first — do not switch branches on their behalf.

   Only when preflight passes (or landed metadata cleanup made the main tree clean), continue.

3. **Merge into the main branch — rebase first, fast-forward second, with submodule-first resolution**

   `speclink/*` branches are local and never pushed, so rewriting their history costs nothing. Replaying the branch onto the target first lets the merge be a fast-forward, which keeps the main branch a straight line instead of collecting one merge node per parallel change.

   Before anything moves, record the main checkout's HEAD — the **pre-merge HEAD**. Step 5 gives it to the quality stations as their starting point in the main checkout:

   ```bash
   git -C <main-checkout> rev-parse HEAD
   ```

   Also inspect whether `160000` submodule gitlinks diverged or changed (`submoduleConflicts`, `fileConflicts`, and `worktree.submodulesChanged` from step 2's preflight, or `git -C <main-checkout> diff --raw "<target-branch>...speclink/<change-name>"`).

   - **When `fileConflicts` is non-empty** (real superproject content conflicts) — stop now or let step 4 abort any attempted merge; never resolve content conflicts yourself.
   - **When `submoduleConflicts` is empty** (no `160000` gitlink diverged on both branches):
     In the worktree, replay the branch onto the target branch verified in step 2:

     ```bash
     git -C <worktree-path> rebase "<target-branch>"
     ```

     - **Rebase succeeds** — in the main checkout, land it without a merge node:

       ```bash
       git -C <main-checkout> merge --ff-only "speclink/<change-name>"
       ```

       If the fast-forward is refused, the target branch moved on between the rebase and the merge — another worktree landed first. Fall back to a plain merge as below, and tell the user this one lands as a merge node.

     - **Rebase reports conflicts** — do **NOT** resolve them. Abort, which restores the branch exactly as it was:

       ```bash
       git -C <worktree-path> rebase --abort
       ```

       Then fall back to a plain merge in the main checkout, and tell the user this one lands as a merge node:

       ```bash
       git -C <main-checkout> merge "speclink/<change-name>"
       ```

       If that merge reports conflicts only on `160000` submodule gitlinks, continue to the submodule-first path below; if any non-submodule file conflicts, go to step 4 immediately.

   - **When `160000` submodule gitlinks diverged (`submoduleConflicts` non-empty, or fallback merge hit only `160000` gitlink conflicts)**:
     Do not abort purely because a solvable `160000` submodule pointer moved on both branches.

     1. If any non-submodule file also conflicts (check `fileConflicts` from preflight, or `git -C <main-checkout> diff --name-only --diff-filter=U` excluding `160000` submodule paths), go straight to step 4 (`merge --abort`) before mutating any submodule.
     2. Otherwise, merge inside each conflicting submodule repo first:
        - Get the worktree's submodule commit SHA: `git -C <main-checkout> rev-parse "speclink/<change-name>:<submodule>"` (`<wt-sub-sha>`).
        - If `git -C <main-checkout>/<submodule> cat-file -e "<wt-sub-sha>^{commit}"` fails, fetch it from `<worktree-path>/<submodule>`:

          ```bash
          git -C <main-checkout>/<submodule> fetch "<worktree-path>/<submodule>" "<wt-sub-sha>"
          ```

        - Merge `<wt-sub-sha>` into `<main-checkout>/<submodule>`'s target branch:

          ```bash
          git -C <main-checkout>/<submodule> merge "<wt-sub-sha>"
          ```

        - If the submodule merge reports real source/content conflicts: run `git -C <main-checkout>/<submodule> merge --abort`, abort any open superproject merge (`git -C <main-checkout> merge --abort`), report the conflicting files inside `<submodule>` verbatim, and go to step 4 to stop.
        - Verify `git -C <main-checkout>/<submodule> status --porcelain` is empty.
     3. Land the superproject merge node and stage the resolved `160000` submodule pointers:
        - If a superproject merge is not already open, run `git -C <main-checkout> merge --no-commit "speclink/<change-name>"`.
        - Verify `git -C <main-checkout> diff --name-only --diff-filter=U` contains only the resolved submodule paths (if any other file conflicts, run `git -C <main-checkout> merge --abort` and go to step 4).
        - Stage each resolved submodule pointer (`git -C <main-checkout> add "<submodule>"`) and complete the merge (`git -C <main-checkout> commit --no-edit`), telling the user this one landed as a merge node with submodule-first resolution.

   - **Sync any remaining moved submodule checkout after landing** — if `speclink/<change-name>` advanced any other `160000` submodule gitlink (in `worktree.submodulesChanged`) that did not go through submodule-first merge above, sync `<main-checkout>/<submodule>` to `HEAD:<submodule>` before removing the worktree: fetch `<wt-sub-sha>` (`git -C <main-checkout> rev-parse "HEAD:<submodule>"`) from `<worktree-path>/<submodule>` if `cat-file -e` fails, then fast-forward/merge `<wt-sub-sha>` onto `<main-checkout>/<submodule>`'s branch when on a named branch (or `git -C <main-checkout> submodule update --init "<submodule>"` when detached) so `<main-checkout>` stays clean.

4. **Conflict — stop immediately**

   If the fallback merge or a submodule merge reports real file/content conflicts:

   - Abort so nothing half-merged is left behind:

     ```bash
     git -C <main-checkout> merge --abort
     ```

     (and `git -C <main-checkout>/<submodule> merge --abort` if a submodule merge conflicted).

   - Report the conflicting file list to the user, verbatim.
   - Do **NOT** edit conflict markers, do **NOT** pick a side, do **NOT** commit a partial merge.
   - STOP and wait for the user to decide how to resolve it.

   The main checkout (and its submodules) must end up exactly as they were before step 3.

5. **Report the landing, carry the quality baseline over, then clean up**

   After a successful merge — fast-forward or merge node alike — the target branch's HEAD is where this change's code landed. Record it first. Take the commit from `git -C <main-checkout> rev-parse HEAD` and pass it, from the main checkout:

   ```bash
   speclink change landed "<change-name>" --commit "<commit>" --branch "speclink/<change-name>"
   ```

   When the verb is refused (for example, a Server that has no such route), tell the user its message and go on with the rest of this step. The merge stays as it is: do not revert it.

   Then, before the worktree is removed, run this from the main checkout:

   ```bash
   speclink review prepare "<change-name>" --base "<pre-merge-HEAD>" --from "<worktree-path>"
   ```

   It writes the quality stations' Apply baseline in the main checkout and carries over both stations' snapshots from the worktree, so `$speclink-review`, `$speclink-verify` and `$speclink-quality` work in the main checkout with no `--base` from the user. From the pre-merge HEAD to the merged HEAD is exactly what this change brought in, conflict resolutions included. If the command fails, report its error and stop here: do not remove the worktree — the snapshots still live there. The merge has already landed, so the way back is not this skill: once the cause is cleared (an open ticket stamped or discarded, a failed copy retried), run that same `speclink review prepare` line again with the pre-merge HEAD recorded in step 3, then run the cleanup below. A second run of this skill would record the merged HEAD as the pre-merge HEAD, and the baseline would then hide every change this merge brought in.

   Then remove the worktree and delete the branch:

   ```bash
   git -C <main-checkout> worktree remove "<worktree-path>"
   git -C <main-checkout> branch -d "speclink/<change-name>"
   ```

   Also delete any merged `speclink/<change-name>` branch inside submodules (`git -C <main-checkout>/<submodule> branch --list "speclink/<change-name>"` → `git -C <main-checkout>/<submodule> branch -d "speclink/<change-name>"`).

   If `worktree remove` refuses because the worktree is dirty, do NOT force it — report what is uncommitted there and STOP. If `branch -d` refuses, report it and STOP: an unmerged commit means step 3 did not carry everything.

   The report updates that change's metadata file in the main checkout; do not leave it uncommitted to block the next parallel worktree merge. Inspect the change's state (`speclink show "<change-name>" --json`, `speclink instructions apply --change "<change-name>" --json`, and `speclink plan --json`):

   - **Ready to finish now** (both quality stamps are already landed and fresh — or the user explicitly chose to skip quality stations — no unchecked `[M]` manual tasks remain, no unstamped tickets exist, and `plan` shows no `archiveAfter` order blocker or conflict): you may run `$speclink-archive <change-name>` in the main checkout and commit all resulting archive changes (deltas merged into specs, change directory moved to archive, and metadata) using the repository's commit convention without `--no-verify`.

   - **Otherwise** (quality stations still need to run in the main checkout, `[M]` tasks remain, or another change archives first): when `git -C <main-checkout> status --porcelain` lists `speclink/changes/<change-name>/.meta.yaml`, commit that landed metadata file explicitly now so the main checkout returns clean before handing off.

     ```bash
     git -C <main-checkout> add "speclink/changes/<change-name>/.meta.yaml"
     git -C <main-checkout> commit -m "<conventional commit recording the landing of <change-name>>"
     ```

     Match the repository's commit convention (for example commitizen / conventional-commit format) and never pass `--no-verify`. Verify `git -C <main-checkout> status --porcelain` is empty.

   After removal the main checkout's `speclink list` no longer shows the `[worktree]` marker for this change, and its task counts read from the main copy again.

6. **Confirm and hand off**

   Tell the user the wrap-up is done: the branch is merged (with submodule-first resolution if applicable), the landing is reported (or its refusal message), the quality baseline is carried over, the worktree and branch are removed, and the main checkout is clean. Then hand off by the change's state. Read the manual tasks from `speclink instructions apply --change "<change-name>" --json` (`remaining` above `codeRemaining` means unchecked `[M]` tasks); ask the user when you cannot tell whether the quality stations already ran.

   - **Already archived in step 5** — report the archive summary and the `next` recommendation from `speclink plan --json`.
   - **The quality stations already ran in the worktree** (their stamps came along with the merge and remain fresh):

     > This change has been merged back into the main branch, and the quality stations already ran in the worktree. Next step: run `$speclink-archive` in the main checkout.

   - **The quality stations have not run (or stamps went stale after a submodule/main merge), and no `[M]` task is open**:

     > This change has been merged back into the main branch, and the quality stations' starting point has been carried over to the main checkout. If you want to run them, run `$speclink-review` (craft quality) ∥ `$speclink-verify` (spec compliance), or `$speclink-quality` (both stations together) right here — whether to run them is your call; then run `$speclink-archive`.

   - **Unchecked `[M]` manual tasks remain**:

     > This change has been merged back into the main branch, and the quality stations' starting point has been carried over to the main checkout. Run `$speclink-review` ∥ `$speclink-verify`, or `$speclink-quality` here, complete and check off the manual tasks, then run `$speclink-archive`.

**Output On Success**

```
## Worktree merged

**Change:** <change-name>
**Branch:** speclink/<change-name> → <main-branch> ✓
**Landed as:** fast-forward (straight line) — or "merge node (rebase fell back)" / "merge node (submodule-first)"
**Submodules:** none — or "<submodule> merged first → <sha> ✓"
**Landing reported:** ✓ — or the verb's refusal message
**Quality baseline:** carried over to the main checkout ✓
**Worktree:** <path> (removed)
**Branch deleted:** ✓
**Main tree:** clean ✓ (or archived ✓)

Next: <step 6 hand-off — already archived → report next from plan; quality stations done → $speclink-archive in the main checkout; not run / invalidated → run the quality stations directly in the main checkout, then archive; unchecked manual tasks remain → run the quality stations in the main checkout, finish manual tasks, then archive>
```

**Output On Stop**

```
## Merge stopped

**Change:** <change-name>
**Reason:** <preflight failure, conflict, or baseline carry failed after the merge landed>

<dirty file list, conflicting file list (including submodule files if applicable), or the prepare error verbatim>

**What you need to do:**
<the specific action — commit/stash in the main tree or submodule, commit in the worktree or submodule, resolve the conflict, or clear the prepare error and rerun that one prepare line with the recorded pre-merge HEAD <sha>, then the cleanup>
```

**Guardrails**

Check these before you report:

- [ ] Never stash or commit unrelated user work on the user's behalf — in the main checkout, submodules, or worktree (step 2)
- [ ] Never resolve merge conflicts yourself; abort the merge (and any failed submodule merge) and report (step 4)
- [ ] Never abort purely on a solvable `160000` submodule gitlink divergence when the submodule repo merges cleanly (step 3)
- [ ] Never resolve rebase conflicts yourself; `rebase --abort` and fall back to the plain merge (step 3)
- [ ] Never leave a half-finished merge state behind in the superproject or any submodule (step 4)
- [ ] Never force-remove a worktree that still has uncommitted work (step 5)
- [ ] The pre-merge HEAD was recorded before the rebase, and the worktree was removed only after `speclink review prepare --base --from` succeeded (steps 3 and 5)
- [ ] Never merge a change whose worktree or submodule you did not verify is fully committed (step 2)
- [ ] Never leave the landed `.meta.yaml` uncommitted in the main checkout after wrap-up (step 5)

## Next steps

Suggestions only. This skill NEVER invokes any of them — report where things stand and stop; the user decides what runs next.

- Already archived in step 5 → report the `next` suggestion from `speclink plan --json`
- The quality stations already ran in the worktree → `$speclink-archive <change-name>` **from the main checkout** — archiving inside a linked worktree is refused by the engine
- The quality stations have not run (or went stale after merge) → run them in the main checkout, where step 5 carried their baseline over: `$speclink-review` ∥ `$speclink-verify`, or `$speclink-quality` for both; then `$speclink-archive <change-name>`
- Unchecked `[M]` manual tasks remain → the quality stations in the main checkout, then the manual tasks, then `$speclink-archive <change-name>`
