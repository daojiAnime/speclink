## Why

使用 oh-my-pi（CLI 指令為 `omp`）進行規格驅動開發的開發者（團隊情境下包含 PO 與 PM），目前在 Speclink 中沒有對應的內建工具目標。既有的內建工具 `pi` 針對原版 Pi（`.pi/skills/`，無子代理與 plan mode），與 oh-my-pi 存在四項實質差異：

1. oh-my-pi 的原生專案技能目錄為 `.omp/skills/`（優先度高於 `.agents/skills/` 與 `.claude/skills/`）；若開發者沿用 `pi` 或 `codex`／`claude` 技能，會落進不正確的目錄或吃到錯誤的叫用前綴與 `--agent` 身分。
2. oh-my-pi 內建專屬工具與子代理體系（互動詢問工具 `ask`、支援批次並行的 `task` 工具與內建代理 `scout`／`reviewer`／`security-reviewer`／`task`／`sonic`、角色模型別名 `@default`／`@slow`／`@smol`／`@task`／`@plan`、寫入工具 `edit`／`write`／`ast_edit`、進度面板 `todo` 與 `local://` plan mode），若只給中性原稿而不附 harness adaptation 前言，模型容易套用 Claude 專有名稱（`AskUserQuestion`、`Agent tool`）或錯失並行子代理與 `analyze` 能力。
3. Herdr 已原生內建獨立的 `omp` agent key（`run: ["omp"]`、`name: "oh-my-pi"`），但 Speclink 的 Herdr 面板僅支援 `claude`、`copilot`、`codex`、`pi`、`cursor`、`antigravity`，無法對 `omp` 發送技能、掃描 `.omp/skills` 或追蹤標籤。
4. 原版 Pi 的標籤追蹤擴充（`pi-labels.ts`）依賴 `session_start` 事件上的 `reason` 欄位，但 oh-my-pi 的 `SessionStartEvent` 不帶 `reason`，切換／新建工作階段由 `session_switch` 事件承載，直接沿用會漏清舊標籤。

本變更作用於工作區初始化（`init`）、技能同步（`update`）、品質關卡子代理模型設定（`workflow-config`）、全套技能執行（含 `analyze`、`propose`、`apply`、`review`、`verify`、`quality`、`audit`、`drift`、`trace` 等）與 Herdr 面板調度。

## What Changes

- **新增獨立內建工具 `omp`（別名 `oh-my-pi`），保留既有 `pi` 不破壞既有使用者**：
  - 技能生成於專案根 `.omp/skills/speclink-<name>/SKILL.md`（不寫入 `~/.omp/agent/skills/` 或任何使用者層設定，不生成指令檔或 `.omp/agents/`）。
  - 技能引用渲染為 `/skill:speclink-<name>`（oh-my-pi 原生叫用語法），不含 `$speclink-` 或 `/speclink-`；`{{TOOL}}` 代換為 `omp`；`{{PLAN_DIR}}` 為空字串。
  - 比照 `claude` 與 `agy` 解鎖全套 19 支技能（含 `analyze`；`worktree` 政策關閉時排除兩顆 worktree 技能）。
  - 技能檔開頭插入 `## oh-my-pi (omp) harness adaptation` 前言，依技能特性映射：
    - **User prompts**：將 AskUserQuestion 映射至 `ask` 工具（不可用時退回純文字詢問）。
    - **Read-only guarantee**（`disallow_edit` 技能）：禁止呼叫 `edit`、`write`、`ast_edit` 修改工作區檔案。
    - **Isolated read sweep**（`analyze`、`drift`、`trace`）：不改檔完成讀取，需要平行掃描時可用 `task` 工具搭配唯讀 `scout` 代理。
    - **Parallel / checking sub-agents**（`audit`、`review`、`verify`、`quality`）：將 `Agent tool` 映射至 `task` 工具（支援單次批次並行派發；`audit` 優先 `security-reviewer`／`scout`，`review` 優先 `reviewer`／`scout`，`verify` 與 `quality` 對應 `reviewer`／`scout` 或唯讀 `task` 代理），並說明如何將 `subagentModel.<station>.omp` 對應至 oh-my-pi 模型角色（`@default`、`@slow`、`@smol`、`@task`、`@plan`）或 `.omp/agents/` 自訂代理定義。
    - **Plan mode input**（`propose`、`ingest`）：將 oh-my-pi plan mode 產生的 `local://<slug>-plan.md` 計畫檔視為計畫來源。
    - **Live progress HUD**（`apply`、`apply-with-worktree`）：可用 `todo` 工具同步進度顯示，並維持 `tasks.md` 為唯一真值來源。
  - 工作區 footprint 偵測不因 `.omp/` 或 `.omp/skills/` 存在而自動推斷 `omp`。
- **工作流設定與內建子代理模型清單（`speclink-core`、`speclink-verbs`）**：
  - `.speclink.yaml` 的 `tools` 清單接受 `omp` 與別名 `oh-my-pi`（不新增設定欄位，不改變預設值）。
  - `speclink/config.yaml` 結構不變；`workflow-config show --json` 回傳的 `builtinSubagentModels` 新增 `"omp": ["@default", "@slow", "@smol", "@task", "@plan"]`（按字母序序列化於既有項目旁）。
- **CLI `speclink init` 與 TUI 多選器（`speclink-cli`）**：
  - `speclink init --tools` 接受 `omp` 與 `oh-my-pi`（大小寫不拘，可與其他工具以逗號組合，無新增旗標、不改 stdin 與 exit code 規則）；非互動缺 `--tools` 單行錯誤、`unknown tool` 錯誤與 `--tools` 說明文字同步列出 `omp`。
  - 互動式 TUI 工具選擇器（`init_tui.rs`）新增 `oh-my-pi` 選項（路徑 `.omp/skills`、叫用提示 `/skill:speclink-*`、自動偵測 PATH 中的 `omp` 執行檔）。
- **Node SDK（`speclink-node`）**：
  - `skills.render` 的 `target` 接受 `'omp'`（含別名 `'oh-my-pi'`），更新 `index.d.ts` 型別聯集與未知 target 錯誤訊息，渲染結果與 CLI 生成的 `.omp/skills/speclink-<name>/SKILL.md` 逐字一致。
- **Desktop（`apps/desktop`）**：
  - `BUILTIN_TOOLS` 與 `BUILTIN_TOOL_NAMES` 新增 `omp`（顯示名 `oh-my-pi`），使初始化確認、啟用確認、checkout 綁定、專案設定 AI 工具頁與子代理模型頁同步支援 `omp`；Tauri 殼 checkout 空選集錯誤訊息與測試同步更新。
- **Herdr 面板整合（`integrations/herdr`）**：
  - `model.js` 的 `SKILL_PREFIX`（`omp: "/skill:"`）、`SPECLINK_TOOL`（`omp: "omp"`）、`ANALYZE_TOOLS`（加入 `"omp"`）、`SKILL_FOLDERS`（掃描專案 `.omp/skills`、`.agents/skills`、`.claude/skills`、`.codex/skills` 與個人 `~/.omp/agent/skills`、`~/.agents/skills`、`~/.claude/skills`）全部支援 `omp`；同時在 `placePlan`／`newTabPlan` 的 `analyze` 不支援提示同步納入 oh-my-pi。
  - 新增 `integrations/herdr/mods/speclink/omp-labels.ts`，針對 oh-my-pi 擴充事件契約追蹤 `input`、`tool_call`（`bash`）與 `session_switch`（`reason === "new" || reason === "resume"` 時清空舊標籤），避免原版 `pi-labels.ts` 監聽無 `reason` 的 `session_start` 而漏清標籤。
  - 同步 `integrations/herdr/README.md`、`integrations/herdr/README.en.md` 與 `scripts/herdr/panel-model.test.mjs`。
- **產物層版號、Golden 與原稿守門（`speclink-core`）**：
  - `ASSET_VERSION` 遞增（由 `v1.63.0` 升至 `v1.64.0`），重生 `assets.lock` 與既有 golden（既有快照僅 `version:` 行變動），新增 `omp.snapshot.md` golden 測試。
  - `skill_authoring.rs` 禁止原稿寫死的 `--agent` 值清單補入 `omp`。
- **中英文文件同步**：
  - 更新 `README.md`、`README.zh-TW.md`、`docs/getting-started.md`、`docs/getting-started.zh-TW.md`、`docs/configuration.md`、`docs/configuration.zh-TW.md`、`docs/workflow.md`、`docs/workflow.zh-TW.md`、`docs/sdk-node.md`、`docs/sdk-node.zh-TW.md`、`docs/product-status.md`、`docs/product-status.zh-TW.md`、`docs/verb-contract.md`、`docs/verb-contract.zh-TW.md`。

### 三種形態下的結果

- **本機 repo（filesystem 模式）**：可用。`speclink init --tools omp`、`speclink update`、Desktop 專案設定與 `skills.render` 完整支援，技能以「規格資料夾位於 checkout」渲染至 `.omp/skills/`；`workflow-config show --json` 回報含 `omp` 的 `builtinSubagentModels`。
- **連到遠端的 checkout（remote 模式）**：可用。`speclink init --store remote --tools omp` 與 `speclink update` 皆可選取 `omp`；遠端 Server 宣告技能時自 Server 取得 `omp` 技能，未宣告或離線時退回本機範本以「規格資料夾不在 checkout」渲染至 `.omp/skills/`；`workflow-config show --json` 由引擎統一回報 `builtinSubagentModels.omp`。
- **沒有本機規格檔的內嵌宿主（Node SDK / `createHost`）**：可用。`skills.render(name, { target: 'omp', specInCheckout: false, ... })` 與 Server 端技能端點皆經同一引擎渲染函式支援 `omp`，並正確省略本機規格路徑與 git 步驟。

### 相容性影響

- **BREAKING（自訂描述子保留名與保留目錄）**：既有的 `.speclink.yaml` 若定義名為 `omp` 或 `oh-my-pi` 的自訂描述子，或 `skills_dir` 詞法正規化後（不分大小寫）等同 `.omp/skills` 的描述子，升級後執行 `speclink update` 會因與內建工具衝突而以非零 exit code 拒絕且零寫入。遷移方式：刪除該自訂描述子，改於 `tools` 列入 `omp`。
- **既有工具與檔案**：原版 `pi`（`.pi/skills/`）與 `claude`、`codex`、`copilot`、`cursor`、`agy` 的技能結構與語意不變；因 `ASSET_VERSION` 遞增至 `v1.64.0`，既有 golden 與已安裝工作區技能檔僅 `version:` 行變動，`speclink update` 可平滑更新。
- **人眼輸出**：`speclink init` 非互動錯誤訊息、`unknown tool` 錯誤訊息、TUI 工具清單與 `Generated files for:` 摘要新增 `omp`／`oh-my-pi`。
- **`--json` 輸出**：`workflow-config show --json` 的 `builtinSubagentModels` 新增 `"omp": ["@default", "@slow", "@smol", "@task", "@plan"]`（既有鍵不變）；技能過期探測在選取 `omp` 時多一筆 `tool: "omp"` 項目，欄位形狀不變。
- **既有工作區升級清理**：升級前手動置於 `.omp/skills/speclink-*` 的目錄，若 `.speclink.yaml` 的 `tools` 清單未列 `omp`，下次 `speclink update` 會依 `speclink-` 前綴所有權視為取消勾選工具而移除（使用者的其他技能目錄不受影響），將於 `docs/configuration` 說明。
- **舊版引擎混用**：舊版 CLI 讀到含 `omp` 的 `.speclink.yaml` 會依既有規則印出 unknown tool 提示並略過，不刪除 `.omp/skills/`。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `workspace-tools`: 內建工具集合納入 `omp`（別名 `oh-my-pi`），新增 oh-my-pi（`omp`）渲染目標需求（含 `.omp/skills`、`/skill:speclink-`、解鎖 `analyze` 與 harness adaptation 前言）；描述子驗證保留 `omp`／`oh-my-pi` 名稱與 `.omp/skills` 目錄；同步更新 init 工具選擇、built-in tools 權威收斂與內建工具渲染差異表。
- `skill-authoring`: 技能引用依工具渲染的範例納入 `omp`（`/skill:speclink-ingest`）；原稿守門禁止寫死的 `--agent` 值清單補入 `omp`。
- `workflow-config`: `workflow-config show --json` 的 `builtinSubagentModels` 納入 `omp` 內建角色清單 `["@default", "@slow", "@smol", "@task", "@plan"]`。
- `node-sdk`: `skills.render` 的 `target` 值域納入 `omp`，並更新錯誤訊息合法值清單。
- `workspace-chooser`: checkout 綁定步驟的工具勾選與空選集條件納入 `omp`，且 `.omp/` 存在不自動預選 `omp`。
- `desktop-config`: 初始化確認、啟用確認、專案設定 AI 工具頁的工具勾選納入 `omp`。
- `user-documentation`: 入門、設定、工作流、SDK、產品狀態與動詞參考中英文文件補入 `omp` 的技能目錄、`/skill:speclink-*` 叫用語法、harness 能力與遷移說明。

## Impact

- 規格掃描結果：掃描 `speclink/specs/` 全部 61 份規格，辨識出 7 份直接相關的既有規格（`workspace-tools`、`skill-authoring`、`workflow-config`、`node-sdk`、`workspace-chooser`、`desktop-config`、`user-documentation`）。所有需求皆為對既有工具矩陣、渲染契約與設定介面的擴展，無須新增獨立 capability。
- Affected specs: workspace-tools, skill-authoring, workflow-config, node-sdk, workspace-chooser, desktop-config, user-documentation
- 影響的技能與工具：為 `omp` 生成全套 19 支技能（含 `analyze`，關閉 worktree 政策時為 17 支）；既有 `claude`、`codex`、`copilot`、`pi`、`cursor`、`agy` 技能內容不變，僅因 `ASSET_VERSION` 遞增而更新 frontmatter 版號行。
- 影響的 crate 與 app：`speclink-core`、`speclink-verbs`、`speclink-cli`、`speclink-node`、`apps/desktop`（前端、`apps/desktop/core`、`apps/desktop/src-tauri`）、`integrations/herdr`。
- Affected code:
  - New:
    - crates/engine/speclink-core/tests/golden/omp.snapshot.md
    - integrations/herdr/mods/speclink/omp-labels.ts
  - Modified:
    - crates/engine/speclink-core/src/workspace/skills.rs
    - crates/engine/speclink-core/src/workspace/config.rs
    - crates/engine/speclink-core/src/workspace/config/tests.rs
    - crates/engine/speclink-core/src/workspace/init.rs
    - crates/engine/speclink-core/src/workspace/init/tests.rs
    - crates/engine/speclink-core/tests/it/render_golden.rs
    - crates/engine/speclink-core/tests/it/skill_authoring.rs
    - crates/engine/speclink-core/tests/it/skill_verbization.rs
    - crates/engine/speclink-core/tests/golden/assets.lock
    - crates/engine/speclink-core/tests/golden/agy.snapshot.md
    - crates/engine/speclink-core/tests/golden/claude.snapshot.md
    - crates/engine/speclink-core/tests/golden/claude-remote.snapshot.md
    - crates/engine/speclink-core/tests/golden/claude-worktree.snapshot.md
    - crates/engine/speclink-core/tests/golden/codex.snapshot.md
    - crates/engine/speclink-core/tests/golden/copilot.snapshot.md
    - crates/engine/speclink-core/tests/golden/cursor.snapshot.md
    - crates/engine/speclink-core/tests/golden/pi.snapshot.md
    - crates/engine/speclink-core/tests/golden/neutral-cli.snapshot.md
    - crates/engine/speclink-core/tests/golden/neutral-tool-call.snapshot.md
    - crates/engine/speclink-core/tests/golden/neutral-tool-call-remote.snapshot.md
    - crates/adapters/speclink-verbs/src/render/workflow_config.rs
    - crates/adapters/speclink-cli/src/verbs/init.rs
    - crates/adapters/speclink-cli/src/verbs/init_tui.rs
    - crates/adapters/speclink-cli/tests/it/init_tools.rs
    - crates/adapters/speclink-cli/tests/it/workflow_config.rs
    - crates/adapters/speclink-node/src/render.rs
    - crates/adapters/speclink-node/index.d.ts
    - crates/adapters/speclink-node/__test__/render.spec.ts
    - apps/desktop/src/builtinTools.ts
    - apps/desktop/src/__tests__/helpers/toolCheckboxes.ts
    - apps/desktop/src/__tests__/App.test.tsx
    - apps/desktop/src/__tests__/workspaceChooser.test.tsx
    - apps/desktop/src/__tests__/projectSettingsView.test.tsx
    - apps/desktop/src-tauri/src/connections.rs
    - apps/desktop/core/src/settings.rs
    - apps/desktop/core/src/project.rs
    - integrations/herdr/mods/speclink/model.js
    - integrations/herdr/mods/speclink/i18n.js
    - integrations/herdr/README.md
    - integrations/herdr/README.en.md
    - scripts/herdr/panel-model.test.mjs
    - README.md
    - README.zh-TW.md
    - docs/getting-started.md
    - docs/getting-started.zh-TW.md
    - docs/configuration.md
    - docs/configuration.zh-TW.md
    - docs/workflow.md
    - docs/workflow.zh-TW.md
    - docs/sdk-node.md
    - docs/sdk-node.zh-TW.md
    - docs/product-status.md
    - docs/product-status.zh-TW.md
    - docs/verb-contract.md
    - docs/verb-contract.zh-TW.md
  - Removed: (none)
