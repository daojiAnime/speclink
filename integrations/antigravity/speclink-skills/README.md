# speclink-skills for Antigravity (`agy`)

Antigravity CLI / App 配套插件，配合 `speclink init --tools agy` 生成的 `.agents/skills/speclink-*` 一起使用，补齐以下原生能力：

- **只读子代理（`agents/`）**：
  - `speclink-reviewer`：用于 `/speclink-review` 与 `/speclink-quality` 的只读审查子代理（仅允许读、搜与沙箱内只读 `run_command`，禁止写入工具）。
  - `speclink-verifier`：用于 `/speclink-verify` 与 `/speclink-quality` 的只读规格验收子代理。
  - `speclink-auditor`：用于 `/speclink-audit` 三视角安全审计的只读子代理。
- **生命周期 Hook（`hooks.json` + `scripts/herdr-label.mjs`）**：
  - 在 `SessionStart`、`PreInvocation` 与 `PostToolUse`（`run_command|Skill|Bash`）自动同步 Herdr 终端标签（`speclink.skill` / `speclink.change` / `speclink.discussion`），并记录 `$TMPDIR/speclink-agy-focus/<conversationId>.json`。
- **条件规则（`rules/speclink-workflow.md`）**：
  - 仅在命中 `.speclink.yaml`、`speclink/**` 或 `docs/specs/**` 时按 glob 触发，零侵入非 Speclink 仓库。
- **状态栏焦点辅助脚本（`scripts/statusline-focus.sh`）**：
  - 从 `agy` `statusLine` / `title` stdin JSON 读取 `conversation_id`，输出当前聚焦的 `<step> · <change>`。

## 校验与安装

```bash
agy plugin validate ./integrations/antigravity/speclink-skills --strict
agy plugin install ./integrations/antigravity/speclink-skills
```
