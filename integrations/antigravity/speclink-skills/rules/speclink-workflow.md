---
trigger: glob
globs: ".speclink.yaml, speclink/**, docs/specs/**"
description: Antigravity (agy) conventions for Speclink repositories
---

# Speclink in Antigravity (`agy`)

- Keep spec documents, `.meta.yaml` stamps, `.evidence.json`, and `config.yaml` authoritative through the `speclink` CLI (`speclink list --json`, `speclink show <name> --json`, `speclink artifact cat ...`, `speclink task done ...`, `speclink review` / `speclink verify`) rather than hand-editing internal state files.
- When a Speclink skill references **AskUserQuestion**, use `ask_question` (or follow any active Herdr callback loop protocol if injected in the session).
- When a Speclink skill spawns parallel read-only checks (`/speclink-review`, `/speclink-verify`, `/speclink-quality`, `/speclink-audit`), prefer `invoke_subagent` with `speclink-reviewer`, `speclink-verifier`, or `speclink-auditor` (or built-in `research` / `self`), running independent checks in one turn and waiting via Reactive Wakeup.
- When mirroring `apply` progress into `update_steps`, still check off each completed task via `speclink task done --change "<name>" --task <n>` — `tasks.md` remains the single source of truth.
