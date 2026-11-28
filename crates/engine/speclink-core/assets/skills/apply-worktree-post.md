
---

## Worktree wrap-up

Once the apply flow above has finished (all tasks complete, or the user stopped you at a good point), do these — still **inside the worktree**.

The apply body's own **Next steps** section does not apply here — this worktree flow replaces it with the edges in **Next steps** at the end of this document.

### W1. Commit the change in the worktree

{{#SPEC_IN_CHECKOUT}}
Follow the `/speclink:commit` skill's attribution convention: stage only the files belonging to this change (its artifacts under `{{SPEC_DIR}}changes/<change-name>/` plus the source files that `speclink evidence show "<change-name>" --json` lists in `files`), leave unrelated dirty files alone, and put the commit message in the project's language.
{{/SPEC_IN_CHECKOUT}}
{{^SPEC_IN_CHECKOUT}}
Follow the `/speclink:commit` skill's attribution convention: stage only the files belonging to this change — the source files that `speclink evidence show "<change-name>" --json` lists in `files`; the spec documents are on the Server and are not part of this commit — leave unrelated dirty files alone, and put the commit message in the project's language.
{{/SPEC_IN_CHECKOUT}}

If any touched files belong to a git submodule (check `git submodule status` or `git diff --raw`), commit those changed files inside each affected submodule first — ensuring that submodule is on branch `speclink/<change-name>` (check `git -C "<submodule-path>" branch --list "speclink/<change-name>"`: check it out if present, otherwise `git -C "<submodule-path>" checkout -b "speclink/<change-name>"`; never leave new commits dangling on a detached HEAD) — then stage the updated `160000` submodule gitlink path in the superproject alongside the superproject files.

The commit lands on branch `speclink/<change-name>` inside the worktree. Nothing reaches the main branch yet.

### W2. Catch up with the main branch, then reset the quality starting point

Check active sibling worktrees and structural overlaps first:

```bash
speclink worktree status --json
speclink plan --json
git worktree list --porcelain
```

Note whether concurrent overlapping worktrees exist — active sibling `speclink/*` worktrees (in `worktrees` / `git worktree list --porcelain`) that share `overlaps` capabilities or `requirementOverlap` in `plan`, or share `sharedPaths` / `sharedSubmodules` in `overlappingWorktrees`.

Rebasing now catches the branch up when clean, and means a stamp landed in a standalone worktree covers the content as it will be after the merge. Do it only while neither quality station has a ticket — a ticket means the stations are already running, and a rebase in the middle of their loop would move the ground under them:

```bash
speclink review show "<change-name>" --json
speclink verify show "<change-name>" --json
```

Both report that no ticket exists → continue. Either one shows a ticket → skip this whole step and say so.

1. Find the main checkout's path in `git worktree list --porcelain` (its first entry), then the branch it is on:

   ```bash
   git -C <main-checkout> branch --show-current
   ```

   A `speclink/*` branch or empty output (detached HEAD) → skip the rest of this step and tell the user why.

2. Record that branch's current commit, then replay this worktree's branch onto it:

   ```bash
   git -C <main-checkout> rev-parse HEAD
   git rebase "<recorded-commit>"
   ```

   - **Conflicts** (including `160000` submodule gitlink conflicts when both branches moved a submodule pointer) → do not resolve them. Run `git rebase --abort` (the branch returns exactly as it was), report the conflicting files, and skip the reset below: the branch did not change, so its old starting point still holds, and `/speclink:worktree-merge` handles submodule-first resolution when landing.
   - **Git refuses for another reason** (for example, uncommitted files) → report git's message and skip the reset below.

3. The rebase succeeded → if any unmodified submodule inside the worktree is clean and its `160000` gitlink moved on the rebased base, sync it (`git submodule update --init "<submodule-path>"`), then move the quality stations' starting point to the recorded commit:

   ```bash
   speclink review prepare "<change-name>" --base "<recorded-commit>"
   ```

   If it fails, report its error and the recorded commit. The branch now sits on the recorded commit while the starting point still names the old fork point, so a quality round run now would count the main branch's own changes as this change's. Rerun that one line once the cause is cleared, before any quality station runs.

A stamp can still go stale if the main branch or an overlapping sibling worktree lands changes to the same files or submodules later.

### W3. Stop here — do not merge, do not remove the worktree

This skill's job ends at the commit. Explicitly:

- **Do NOT** merge `speclink/<change-name>` back into the main branch.
- **Do NOT** run `git worktree remove`, and do NOT delete the branch.
- **Do NOT** switch the main checkout to another branch or touch it in any way.

Merging is a separate, human-triggered step: it needs a clean main tree, and a conflict there is the user's call, not yours. Leaving the worktree in place also means the main checkout's `speclink list` keeps showing this change with its `[worktree]` marker until the merge happens.

### W4. Hand off

Pick the path by the change's state: unchecked `[M]` manual tasks show in `speclink instructions apply --change "<change-name>" --json` as `remaining` above `codeRemaining`, and W2 checked whether concurrent overlapping sibling worktrees exist (sharing `overlaps`, `requirementOverlap`, `sharedPaths`, or `sharedSubmodules`).

- **Concurrent overlapping worktrees exist** — advise deferring quality stamping until after merging back into the main checkout (name the overlapping sibling worktrees, shared capabilities, or shared submodules):

  > This change is finished and committed inside the worktree, and has not been merged back into the main branch yet.
  >
  > Concurrent overlapping worktrees are active (<overlapping sibling names / shared capabilities or submodules>), so defer `/speclink:review` (craft quality) ∥ `/speclink:verify` (spec compliance), or `/speclink:quality` (both stations together), until after `/speclink:worktree-merge <change-name>` back in the main checkout — stamps created on this fork-point commit would be invalidated as soon as another overlapping branch lands first.
  >
  > Suggested next step: run `/speclink:worktree-merge <change-name>` first — it handles submodule-first resolution if needed, carries the quality stations' starting point over to the main checkout, removes the worktree and branch, and returns the main tree clean. Then run `/speclink:review` ∥ `/speclink:verify`, or `/speclink:quality` in the main checkout, finish any manual tasks, and run `/speclink:archive`.

- **No unchecked `[M]` task and no concurrent overlapping worktrees** — tell the user, plainly:

  > This change is finished and committed inside the worktree, and has not been merged back into the main branch yet.
  >
  {{#SPEC_IN_CHECKOUT}}
  > Suggested next step: run the quality stations in this worktree — `/speclink:review` (craft quality) ∥ `/speclink:verify` (spec compliance), or `/speclink:quality` (both stations together); whether to run them is your call. Stamping writes to the change's metadata, so remember to make one more commit, then run `/speclink:worktree-merge <change-name>` to wrap up.
  {{/SPEC_IN_CHECKOUT}}
  {{^SPEC_IN_CHECKOUT}}
  > Suggested next step: run the quality stations in this worktree — `/speclink:review` (craft quality) ∥ `/speclink:verify` (spec compliance), or `/speclink:quality` (both stations together); whether to run them is your call. Once stamped, run `/speclink:worktree-merge <change-name>` to wrap up (stamps live on the Server, so no extra commit is needed).
  {{/SPEC_IN_CHECKOUT}}
  >
  > If you want to see CI results before stamping, take the other route: run `/speclink:worktree-merge <change-name>` to merge first, then run the quality stations in the main checkout — worktree branches stay local and are never pushed, so CI cannot see them; merging carries the quality stations' starting point over to the main checkout.

- **Unchecked `[M]` tasks remain (with no concurrent overlap)** — tell the user, plainly:

  > This change is finished and committed inside the worktree, has not been merged back into the main branch yet, and still has unchecked manual tasks.
  >
  > Suggested next step: run `/speclink:worktree-merge <change-name>` first — it checks that the main tree is clean, merges the branch back, carries the quality stations' starting point over to the main checkout, and removes the worktree and branch on success. After that, run the quality stations in the main checkout — `/speclink:review` (craft quality) ∥ `/speclink:verify` (spec compliance), or `/speclink:quality` (both stations together); whether to run them is your call — then finish the manual tasks and archive.

Report alongside it: the worktree path, the branch name, the tasks completed this session, any overlapping sibling worktrees detected, and whether W2 rebased or why it skipped.

### Wrap-up check

Check these before you hand off:

{{#SPEC_IN_CHECKOUT}}
- [ ] The commit in the worktree (and inside any modified submodule on `speclink/<change-name>`) holds only this change's artifacts and evidence-recorded source files (W1).
{{/SPEC_IN_CHECKOUT}}
{{^SPEC_IN_CHECKOUT}}
- [ ] The commit in the worktree (and inside any modified submodule on `speclink/<change-name>`) holds only this change's evidence-recorded source files — no spec document (W1).
{{/SPEC_IN_CHECKOUT}}
- [ ] W2 rebased only while neither station had a ticket; a conflict was aborted, never resolved; a successful rebase was followed by `speclink review prepare --base` (W2).
- [ ] Nothing was merged, no worktree or branch was removed, and the main checkout was not touched (W3).
- [ ] The hand-off names the worktree path, the branch and the tasks completed, advises post-merge quality stamping when concurrent overlapping worktrees exist or `[M]` tasks remain, and lists review, verify and quality as the entries next to `/speclink:worktree-merge` (W4).

## Next steps

{{NEXT_STEPS_LEAD}}

- Concurrent overlapping sibling worktrees exist, unchecked `[M]` tasks remain, or the user wants CI results before stamping (the worktree branch is local only, so CI never sees it) → `/speclink:worktree-merge <change-name>` first, then the quality stations (`/speclink:review` ∥ `/speclink:verify`, or `/speclink:quality`) and any manual tasks in the main checkout, then archive
{{#SPEC_IN_CHECKOUT}}
- No unchecked `[M]` task and no concurrent overlapping sibling worktrees → the quality stations here in the worktree: `/speclink:review` ∥ `/speclink:verify`, or `/speclink:quality` for both; commit the stamps, then `/speclink:worktree-merge <change-name>`
{{/SPEC_IN_CHECKOUT}}
{{^SPEC_IN_CHECKOUT}}
- No unchecked `[M]` task and no concurrent overlapping sibling worktrees → the quality stations here in the worktree: `/speclink:review` ∥ `/speclink:verify`, or `/speclink:quality` for both; then `/speclink:worktree-merge <change-name>` (the stamps are on the Server — nothing to commit for them)
{{/SPEC_IN_CHECKOUT}}
- Requirements changed mid-work → `/speclink:ingest <change-name>` inside this worktree, then resume apply here
