## Context

Speclink 目前在 `speclink-core` 內建六個 AI 工具（`Tool::ALL = [Claude, Codex, Copilot, Pi, Cursor, Agy]`），由 `Tool::ALL` 統一驅動技能生成、取消勾選清理、自訂描述子保留名／保留目錄驗證、`assets.lock` 指紋計算與錯誤訊息清單（`Tool::supported_names`）。其中：

- `pi` 對應原版 Pi（`.pi/skills/`，叫用前綴 `/skill:speclink-`，無子代理與 plan mode，不解鎖 `analyze`，不附 harness 前言）。
- `agy` 對應 Antigravity（`.agents/skills/`，叫用前綴 `/speclink-`，解鎖 `analyze`，附 `agy_context(skill)` harness adaptation 前言，並在 `BUILTIN_SUBAGENT_MODELS` 宣告固定模型層級）。
- `apps/desktop` 目前 `BUILTIN_TOOLS` 僅列五個工具（`claude`、`codex`、`copilot`、`pi`、`cursor`），本變更只聚焦補齊 `omp`，不順手改動 `agy` 的桌面狀態。

經查證本機安裝的 oh-my-pi（`@oh-my-pi/pi-coding-agent`）實作與 Herdr 內建設定：

- 專案與個人技能目錄：oh-my-pi 預設優先掃描專案 `.omp/skills/`（向上回溯祖先目錄），其次掃描相容目錄 `.agents/skills/`、`.claude/skills/`、`.codex/skills/`、`.github/skills/`，個人層為 `~/.omp/agent/skills/`（亦掃描 `~/.agents/skills/`、`~/.claude/skills/`）。因此將技能寫在 `.omp/skills/` 可在與 `claude`／`codex`／`agy` 共存時穩定勝出。
- 技能叫用語法：oh-my-pi 的斜線叫用語法為 `/skill:<name>`（`getSkillSlashCommandName`），不支援 bare `/<name>` 呼叫技能。
- 內建工具與子代理能力：互動詢問工具為 `ask`；子代理工具為 `task`（支援單筆呼叫或 `tasks` 批次並行），內建代理含唯讀探索 `scout`（`@smol`）、程式碼審查 `reviewer`（`@slow`）、唯讀安全審查 `security-reviewer`、通用 `task`（`@task`）與機械快速代理 `sonic`（`@smol`），並支援專案 `.omp/agents/*.md` 與個人 `~/.omp/agent/agents/*.md`；寫入工作區檔案的工具為 `edit`、`write`、`ast_edit`；進度清單工具為 `todo`；plan mode 產物使用 `local://<slug>-plan.md`。
- 模型角色別名：`MODEL_ROLE_IDS` 定義版本無關的角色別名（`@default`、`@slow`、`@smol`、`@task`、`@plan` 等，底層映射到使用者於 `config.yml` 設定的模型優先鏈）。
- Herdr 整合：Herdr 已內建獨立的 `"omp"` agent key（`name: "oh-my-pi"`、`run: ["omp"]`、`match: ["oh-my-pi"]`），且 `omp` CLI 直接接受位置參數傳入初始提示詞。
- 擴充事件契約：oh-my-pi 擴充提供 `input`（`event.text`）、`tool_call`（`event.toolName === "bash"`、`event.input.command`）與 `session_switch`（`event.reason === "new" | "resume" | "fork"`）；`session_start` 事件不帶 `reason` 欄位。

## Goals / Non-Goals

**Goals:**

- 以獨立內建工具 `omp`（別名 `oh-my-pi`）完整支援 oh-my-pi，保留原版 `pi`（`.pi/skills/`）的既有行為與向後相容性。
- 在 `speclink-core` 實作唯一真值來源，同步貫穿本機 repo、遠端 checkout、Node SDK／內嵌宿主、CLI／TUI、Desktop 與 Herdr 面板，不建立平行邏輯。
- 提供針對 oh-my-pi 真實工具集的 `omp_context(skill)` harness adaptation 前言、解鎖 `analyze` 技能，並於 `BUILTIN_SUBAGENT_MODELS` 提供版本無關的 oh-my-pi 角色清單。
- 提供符合 oh-my-pi 事件契約的 Herdr 標籤追蹤擴充（`omp-labels.ts`），修復工作階段切換時清標籤的語意差異。

**Non-Goals:**

- 不改寫既有 `pi` 工具的目錄、前綴或行為，不將 `pi` 轉向 oh-my-pi。
- 不產生指令檔（`AGENTS.md`、`CLAUDE.md`、`.omp/rules/`），也不產生 `.omp/agents/*.md` 自訂代理檔或使用者層 `~/.omp/agent/skills/`。
- 不因 `.omp/` 或 `.omp/skills/` 存在而在 footprint 偵測自動推斷 `omp`。
- 不在本變更中補做 `agy` 的 Desktop UI 或本專案自身 `.speclink.yaml` 的工具選取（保持變更邊界乾淨）。
- 不新增或修改 `@speclink/ui` 元件或變體；Desktop 直接沿用既有的 checkbox／switch／列表元件。
- 不修改規格儲存契約（`speclink-store-port`）或任何非工具矩陣相關的遠端協定端點。
- 不自動安裝或改寫使用者本機的 `~/.omp/agent/extensions/` 或 Herdr 本機 mods 目錄。

## Decisions

### 1. 內建工具列舉新增 Omp 與保留目錄邊界

**落點**：`crates/engine/speclink-core/src/workspace/skills.rs`、`crates/engine/speclink-core/src/workspace/init.rs`

在 `speclink-core` 的 `Tool` 列舉新增 `Tool::Omp`，將 `Tool::ALL` 擴為 7 個元素：`[Claude, Codex, Copilot, Pi, Cursor, Agy, Omp]`。各方法對應如下：

| 變體 | `parse` 接受字串 | `name()` | `skills_dir()` | `plan_dir()` | `slash_replacement()` |
| --- | --- | --- | --- | --- | --- |
| `Omp` | `"omp"`、`"oh-my-pi"`（trim＋小寫化） | `"omp"` | `".omp/skills"` | `""` | `"/skill:speclink-"` |

- `instructions_path(Tool::Omp)` 回傳 `None`（從未注入指令檔，剝除迴圈跳過）。
- `detect_footprint_tools` 不掃描 `.omp/`，避免專案僅放 `.omp/rules/` 或個人設定時被誤預選。
- `ToolDescriptor::validate` 透過 `Tool::parse` 與 `Tool::ALL` 自動拒絕名為 `omp`／`oh-my-pi` 或詞法正規化後（不分大小寫）等同 `.omp/skills` 的自訂描述子；`.omp/rules`、`.omp/agents` 等不同目錄仍合法。
- **跨形態共用**：本機 `init`／`update`、遠端 `init_remote`／Server `skills` 端點與 Node SDK `skills.render` 全數共用同一份 `Tool` 與 `render_skill_file_for`，沒有平行實作。
- **寫入順序與失敗可觀察狀態**：`SyncPlan::guard`（共享 symlink 檢查＋降版守門）在任何寫入（含 `.speclink.yaml` 與 `.omp/skills/`）之前執行，驗證失敗即零寫入；寫入階段按既有 `SyncPlan::apply` 冪等順序寫入 `SKILL.md` 再清理孤兒 `speclink-*` 目錄，重試可收斂。
- **替代方案**：修改現有 `Tool::Pi` 指向 `.omp/skills`。否決理由：破壞既有原版 Pi 使用者（`.pi/skills` 與 Herdr `pi` key），違反向後相容。

### 2. oh-my-pi Harness Adaptation 前言與解鎖 analyze

**落點**：`crates/engine/speclink-core/src/workspace/skills.rs`

比照 `agy_context(skill)` 新增 `omp_context(skill)`，在 `render_skill_file_with_limits` 的 `RenderTarget::Builtin(Tool::Omp)` 分支（位於 frontmatter 之後、`## Harness limits` 與對話語言段落之前）插入 `## oh-my-pi (omp) harness adaptation` 前言，同時將 `Skill::is_supported_by` 擴展為 `matches!(target, RenderTarget::Builtin(Tool::Claude | Tool::Agy | Tool::Omp))` 以解鎖 `analyze`。

`omp_context(skill)` 依技能屬性精準組合提示條目：

- 所有技能必含：**User prompts** — 將 **AskUserQuestion tool** 映射至 `ask` 工具（不可用時以純文字提問並等待回覆）。
- `skill.disallow_edit` 為真：**Read-only guarantee** — 禁止呼叫 `edit`、`write`、`ast_edit` 修改工作區檔案，只進行唯讀檢查與必要 `speclink` CLI 動詞。
- `analyze`／`drift`／`trace`：**Isolated read sweep** — 在不改檔前提下完成檢查，需要平行唯讀掃描時可使用 `task` 工具派發 `scout` 代理。
- `audit`：**Parallel sub-agents** — 透過 `task` 工具的批次派發同時啟動多個審查視角（優先選用 `security-reviewer` 或唯讀 `scout` 代理，亦可使用 `.omp/agents/` 自訂唯讀代理）。
- `review`：**Checking sub-agents** — 將 `Agent tool` 映射至 `task` 工具並行派發 Standards 與 Correctness 檢查（優先 `reviewer` 或唯讀 `scout` 代理），並說明讀取 `subagentModel.review.omp`（角色別名 `@default`、`@slow`、`@smol`、`@task`、`@plan` 或自訂代理設定）。
- `verify`：**Checking sub-agent** — 將 `Agent tool` 映射至單一唯讀 `task` 呼叫（優先 `reviewer`／`scout` 或唯讀限制的 `task` 代理），讀取 `subagentModel.verify.omp`。
- `quality`：**Parallel stations** — 在同一回合以 `task` 工具並行派發 review Standards、review Correctness 與 verify 檢查，依 `subagentModel.<station>.omp` 對應模型角色或自訂代理。
- `propose`／`ingest`：**Plan mode input** — 在 oh-my-pi plan mode 下，將對話中出現的 `local://<slug>-plan.md` 計畫檔視為計畫來源，而非期待固定目錄。
- `apply`／`apply-with-worktree`：**Live progress HUD** — 可將進度唯讀映射至 `todo` 工具，同時維持 `tasks.md`（`speclink task done`）為唯一真值來源。

frontmatter 維持中性欄位（`name`、`description`、`license`、`compatibility`、`metadata`），不輸出 Claude 專用的 `context`、`agent`、`disallowedTools`。

- **替代方案**：不加前言、走純 `pi`／`cursor` 路線。否決理由：使用者已明確要求比照 `agy` 完整適配；oh-my-pi 具備 `ask`、`task`、`scout`／`reviewer`、`todo` 與 `local://` plan mode，缺少前言會讓模型呼叫錯誤工具名或降級為單執行緒執行。

### 3. 內建子代理模型清單新增 omp 角色別名

**落點**：`crates/engine/speclink-core/src/workspace/config.rs`、`crates/adapters/speclink-verbs/src/render/workflow_config.rs`

在 `speclink-core` 的 `BUILTIN_SUBAGENT_MODELS` 常數新增：

```rust
("omp", &["@default", "@slow", "@smol", "@task", "@plan"])
```

- **唯一實作落點**：`speclink-verbs` 的 `workflow-config show`（本機 CLI、遠端 CLI、Node SDK `dispatch` 與 Desktop IPC）皆讀取 `BUILTIN_SUBAGENT_MODELS`，轉換為 `BTreeMap` 後輸出 `--json` 欄位 `builtinSubagentModels`（camelCase，鍵按字母序為 `agy`、`claude`、`omp`）。人眼輸出不顯示 `subagent_model`，維持位元級不變。
- **YAML 序列化與向後相容**：`speclink/config.yaml` 的 serde 結構（`SubagentModelConfig`）完全不變，既有設定檔照常解析；當使用者於 `subagent_model.review.omp` 寫入 `@slow` 等以 `@` 開頭的純量時，既有 `update_workflow_config_text`（經 `serde_yaml` 序列化與重解析驗證）會自動以引號跳脫 `"@slow"` 並無損 round-trip。
- **替代方案**：不加進 `BUILTIN_SUBAGENT_MODELS`，讓使用者全部手填 `choices`。否決理由：oh-my-pi 的角色別名與具體模型版號解耦、不會隨模型改版過期，與 Claude 的 `sonnet/opus/haiku/fable` 及 Agy 的 `inherit/pro/flash/flash_lite` 屬於同一類內建清單。

### 4. CLI init 與 TUI 工具選擇器接入 omp

**落點**：`crates/adapters/speclink-cli/src/verbs/init.rs`、`crates/adapters/speclink-cli/src/verbs/init_tui.rs`

- `InitArgs.tools` help 範例與非互動缺少 `--tools` 的單行錯誤常數同步納入 `omp`。
- `init_tui::collect_context` 在 `options` 清單新增 `Tool::Omp`：
  - `label`: `"oh-my-pi"`
  - `skills_dir`: `".omp/skills"`（由 `Tool::Omp.skills_dir()` 取得）
  - `invoke_hint`: `"/skill:speclink-*"`
  - `description`: `"Generate .omp/skills/ (/skill:speclink-*) with oh-my-pi harness adaptation."`
  - `bins`: `&["omp"]`（偵測本機 PATH 是否有 `omp`）
- 子指令、旗標、stdin、exit code 與 stdout／stderr 分流契約不變。

### 5. Node SDK 渲染目標擴展

**落點**：`crates/adapters/speclink-node/src/render.rs`、`crates/adapters/speclink-node/index.d.ts`

- `resolve_matrix` 既已透過 `Tool::parse` 解析內建工具，自動接受 `"omp"` 與 `"oh-my-pi"`；更新未知 target 的錯誤訊息，於合法值清單加入 `'omp'`。
- `index.d.ts` 的 `RenderOptions.target` 型別聯集新增 `'omp'`，並同步更新 `docs/sdk-node.md` 與 `docs/sdk-node.zh-TW.md`。

### 6. Desktop 勾選清單與 Tauri 綁定邊界

**落點**：`apps/desktop/src/builtinTools.ts`、`apps/desktop/src-tauri/src/connections.rs`、`apps/desktop/core/src/settings.rs`、`apps/desktop/core/src/project.rs`

- 前端單一落點：`BUILTIN_TOOLS` 加入 `"omp"`，`BUILTIN_TOOL_NAMES` 加入 `omp: "oh-my-pi"`。初始化確認、啟用確認、checkout 綁定、專案設定 AI 工具頁與子代理模型頁四處 UI 與設定頁表格皆讀取同一常數，自動呈現 `omp`；全部沿用既有 `@speclink/ui` 元件（無新增元件或變體）。
- 邏輯邊界：`apps/desktop/core` 經 `speclink_core::init::parse_tool_names` 與 `reconcile_builtin_tools` 處理，只需補齊測試；`apps/desktop/src-tauri/src/connections.rs` 的空選集繁中錯誤訊息與註解同步加入 `omp`，並補齊 `.omp/` 不被預選及綁定 `[claude, omp]` 的單元測試。

### 7. Herdr 面板整合與 oh-my-pi 標籤追蹤擴充

**落點**：`integrations/herdr/mods/speclink/model.js`、`integrations/herdr/mods/speclink/i18n.js`、`integrations/herdr/mods/speclink/omp-labels.ts`

- `model.js`：
  - `SKILL_PREFIX` 加入 `omp: "/skill:"`
  - `SPECLINK_TOOL` 加入 `omp: "omp"`
  - `ANALYZE_TOOLS` 加入 `"omp"`，同步更新 `placePlan`／`newTabPlan` 的不支援提示文字（及 `i18n.js` 對應詞條）為包含 oh-my-pi
  - `SKILL_FOLDERS` 加入 `omp`：`[["project", ".omp/skills"], ["project", ".agents/skills"], ["project", ".claude/skills"], ["project", ".codex/skills"], ["personal", "~/.omp/agent/skills"], ["personal", "~/.agents/skills"], ["personal", "~/.claude/skills"]]`
  - `agentArgv` 不需特例：`omp` 直接以 `pick.run`（`["omp"]`）接位置提示詞啟動。
- `omp-labels.ts`（新建）：
  - 監聽 `pi.on("input", ...)` → 轉 `UserPromptSubmit` 並帶 `prompt: event.text`
  - 監聽 `pi.on("tool_call", ...)` → `event.toolName === "bash"` 時轉 `Bash` 並帶 `command: event.input?.command`
  - 監聽 `pi.on("session_switch", ...)` → `event.reason === "new" || event.reason === "resume"` 時轉 `SessionStart`＋`source: "clear"` 清空舊標籤（修正 `pi-labels.ts` 監聽無 `reason` 的 `session_start` 在 oh-my-pi 下失效的問題）
  - 呼叫 `hc label` 仍受 `try/catch` 與 3 秒 timeout 保護，不在 Herdr 內或失敗時靜默不阻斷。
- 不變動原版 `pi-labels.ts`，維持原版 Pi 向後相容。

### 8. 原稿守門、產物版號、Golden 與文件同步

**落點**：`crates/engine/speclink-core/src/workspace/init.rs`、`crates/engine/speclink-core/tests/it/render_golden.rs`、`crates/engine/speclink-core/tests/it/skill_authoring.rs`、`crates/engine/speclink-core/tests/it/skill_verbization.rs`、`crates/engine/speclink-core/tests/golden/`、`docs/`、`README*.md`

- `ASSET_VERSION` 由 `"v1.63.0"` 遞增為 `"v1.64.0"`，重生 `assets.lock` 與全部既有 golden（確認既有快照僅 `version:` 行改變），新增 `omp_rendering_is_bit_identical_to_golden` 與 `omp.snapshot.md`。
- `skill_authoring.rs` 的寫死 `--agent` 值偵測正規式納入 `omp`，補測試確認 `--agent omp` 會被攔截。
- `skill_verbization.rs` 若枚舉內建工具則同步涵蓋 `Tool::Omp`。
- 同步中英文文件：`README`、`getting-started`、`configuration`、`workflow`、`sdk-node`、`product-status`、`verb-contract`（新增 `builtinSubagentModels.omp` 欄位值說明）與 `integrations/herdr/README*.md`。
- **儲存解耦說明**：本變更僅擴展工作區工具渲染矩陣與引擎常數，不觸及 `StorePort` 介面或遠端同步演算法，完全維持儲存解耦邊界。

## Implementation Contract

**行為（Behavior）**

- `speclink init --tools omp`（與 `--tools oh-my-pi`、大小寫不拘，含 `--store remote`）：exit code 0，`.speclink.yaml` 的 `tools` 記錄 `omp`，於 `.omp/skills/speclink-<name>/SKILL.md` 生成全套技能（含 `speclink-analyze`；`worktree` 關閉時為 17 支、開啟時為 19 支）；不生成 `.claude/`、`.agents/`、`.pi/`、`AGENTS.md`、`CLAUDE.md`、`.omp/agents/` 或 `~/.omp/` 下任何檔案。
- 生成的 `omp` 技能 frontmatter 僅含 `name`、`description`、`license`、`compatibility`、`metadata`（`author`、`version: "v1.64.0"`、`generatedBy`）；內文開頭帶 `## oh-my-pi (omp) harness adaptation` 前言，技能引用全數為 `/skill:speclink-<name>`（不含 `$speclink-`、`/speclink-` 或 `## Claude fork context`），`{{TOOL}}` 代換為 `omp`。
- `speclink update` 取消勾選 `omp` 時，僅移除 `.omp/skills/` 下 `speclink-` 前綴目錄及因而變空的 `.omp/skills/` 與 `.omp/`；使用者自有技能與 `.omp/` 其他檔案位元級保留，symlink 路徑絕不穿透清理，經 symlink 與其他工具共用目錄者在任何寫入前以單行錯誤拒絕（`--allow-downgrade` 亦不可越過）。
- `.speclink.yaml` 無 `tools` 清單或 Desktop checkout 預選時，`.omp/` 或 `.omp/skills/` 存在不得自動預選 `omp`。
- `speclink workflow-config show --json`（本機、遠端與 Node SDK `dispatch`）的 `builtinSubagentModels` 包含 `"omp": ["@default", "@slow", "@smol", "@task", "@plan"]`；人眼輸出逐位元不變。
- `skills.render(name, { target: 'omp', ... })` 回傳字串與 CLI 生成的 `.omp/skills/speclink-<name>/SKILL.md` 逐字一致；`specInCheckout: false` 時正確剔除本機規格路徑與 git 步驟。
- Desktop 初始化確認、啟用確認、checkout 綁定、專案設定 AI 工具頁與子代理模型頁皆提供 `omp`（顯示名 `oh-my-pi`）。
- Herdr 面板可辨識已安裝的 `omp` agent、送出 `/skill:speclink-<name>` 指令（含 `analyze`）、於缺技能時提示 `speclink init --tools omp`，並透過 `omp-labels.ts` 在 `input`、`bash` `tool_call` 與 `session_switch` 更新／清除終端標籤。

**介面與資料形狀（Interface / Data Shape）**

- `.speclink.yaml`：`tools` 清單新增內建工具名 `"omp"`（接受別名 `"oh-my-pi"`），無新鍵、serde 結構不變。
- `speclink/config.yaml`：serde 結構不變；`subagent_model.{review,verify}.omp` 可接受 `"@default"`、`"@slow"`、`"@smol"`、`"@task"`、`"@plan"` 等字串並正確引號序列化。
- CLI：`speclink init --tools <list>` 接受 `omp`／`oh-my-pi`；`speclink workflow-config show --json` 的 `builtinSubagentModels` 型別仍為 `Record<string, string[]>`，鍵集合加入 `"omp"`。
- Node SDK：`RenderOptions.target` 型別聯集加入 `'omp'`；未知 target 錯誤訊息列出 `'omp'`。
- Herdr：`integrations/herdr/mods/speclink/omp-labels.ts` 匯出預設擴充工廠函式 `export default function (pi: any)`，掛載 `input`、`tool_call`、`session_switch`。

**失敗模式（Failure Modes）**

- 描述子 `name` 為 `omp` 或 `oh-my-pi`，或 `skills_dir` 正規化後（不分大小寫）等於 `.omp/skills`：非零 exit code、stderr 單行語義化錯誤、零寫入、既有檔案逐字元不變。
- 已勾選 `omp` 的 `.omp/skills` 經 symlink 指到另一工具的 skills 目錄：寫入前拒絕、單行錯誤、零寫入（`--allow-downgrade` 不可越過）。
- 非互動執行 `speclink init` 缺 `--tools` 或 `--tools` 含未知名稱：非零 exit code、stderr 單行錯誤列出含 `omp` 的完整工具清單、零寫入。
- `skills.render` 傳入未知 `target`：拋出 Error，訊息列出包含 `'omp'` 的所有合法值。
- `omp-labels.ts` 在非 Herdr 環境（無 `HC_TERMINAL_ID`）或 `hc label` 逾時／失敗時靜默略過，不中斷使用者輸入或工具執行。

**驗收標準（Acceptance Criteria）**

- `cargo test -p speclink-core` 通過（含 `omp.snapshot.md` golden、`assets.lock`、描述子保留名／目錄、symlink 防護、`@slow` YAML round-trip、`skill_authoring` 守門）。
- `cargo test -p speclink-verbs` 與 `cargo test -p speclink-cli` 通過（含 `init`／`init_tui`／`init_tools`／`workflow_config` 與 `remote_verb_parity`）。
- `npm --prefix crates/adapters/speclink-node run build` 與 `npm --prefix crates/adapters/speclink-node test` 通過。
- `cargo test -p speclink-desktop-core`、`cargo test -p speclink-desktop --lib connections` 與 `npm test -w apps/desktop` 通過。
- `node --test scripts/*.test.mjs scripts/*/*.test.mjs` 全部通過（含 `scripts/herdr/panel-model.test.mjs` 與文件對等／連結／詞彙守門）。
- `[M]` 手動於真實 `omp` CLI 工作階段驗證 `/skill:speclink-propose`、`/skill:speclink-analyze` 載入與 `omp-labels.ts` 在 Herdr 中的貼標／切工作階段清標行為。

**範圍邊界（Scope Boundaries）**

- **In scope**：`speclink-core` 工具矩陣／前言／內建模型角色／版號／golden／守門、`speclink-verbs` 渲染測試、`speclink-cli` init 與 TUI、`speclink-node` target 渲染與型別、`apps/desktop`（UI、desktop-core、Tauri 殼）的 `omp` 工具選項、`integrations/herdr` 面板與 `omp-labels.ts`、中英文文件同步。
- **Out of scope**：修改原版 `pi`、在 Desktop 補做 `agy` UI、生成 `.omp/agents/` 或指令檔、寫入 `~/.omp/` 使用者設定、改動本專案根 `.speclink.yaml`、手動修改 `speclink/manual/`（留待 `/speclink-manual` 重生）。

## Risks / Trade-offs

- **[Golden 回歸與既有未提交改動]** `ASSET_VERSION` 遞增至 `v1.64.0` 會觸動全部既有 golden 快照，且目前工作樹中已有未提交的 `agy` 相關調整 → 重生 golden 後須逐檔檢視 diff，確認非 `omp` 快照只動到 `version:` 行（與工作樹既有變更邊界清楚分離），絕不掩蓋意外改動。
- **[跨平台路徑與大小寫比對]** Windows／macOS 檔案系統不區分大小寫且路徑分隔符與換行符（CRLF）不同 → `.omp/skills` 沿用既有 `lexical_normalize`＋小寫摺疊與 `eol_normalized`，所有比對邏輯走統一抽象、不加平台特判，並由 Windows／macOS／Linux CI 覆蓋同一組測試。
- **[YAML `@` 角色純量跳脫]** oh-my-pi 角色別名以 YAML 保留字元 `@` 開頭（如 `@slow`），若未跳脫會導致 YAML 解析失敗 → 沿用既有 `serde_yaml` 序列化與重解析驗證（`verify_rewritten_config`），並以單元測試鎖定 `@slow`／`@smol` 寫入與回讀 round-trip。
- **[BREAKING 自訂描述子]** 升級前曾自建名為 `omp`／`oh-my-pi` 或指向 `.omp/skills` 描述子的專案在 `speclink update` 會被拒 → 錯誤訊息明指衝突，並於 `docs/configuration` 撰寫遷移步驟。
- **[oh-my-pi 多目錄技能共存]** 若同時勾選 `claude`／`codex`／`omp`，oh-my-pi 會掃描多個技能目錄，但因 `.omp/skills/` 在 oh-my-pi 內建 provider 優先度最高，同名技能會優先採用 `.omp/skills/` 的 `omp` 版本 → 在 `docs/getting-started` 說明此優先順序並以 `[M]` 手動實測確認。
- **[Herdr 擴充與多工具提示文案]** `placePlan`／`newTabPlan` 的 `analyze` 提示訊息改為含 oh-my-pi 會影響 `panel-model.test.mjs` 斷言與 `i18n.js` 鍵值 → 同批更新 `i18n.js` 與 `panel-model.test.mjs` 並執行全組 `node --test` 守門。
