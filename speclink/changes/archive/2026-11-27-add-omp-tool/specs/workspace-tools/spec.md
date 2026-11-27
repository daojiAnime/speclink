## ADDED Requirements

### Requirement: oh-my-pi（omp）渲染目標

內建工具 `omp`（別名 `oh-my-pi`，解析時大小寫不拘並修剪前後空白）的技能檔 SHALL 生成於專案根的 `.omp/skills/speclink-<name>/SKILL.md`。生成集合 SHALL 包含全套技能（含 `analyze`，與 `claude` 相同；worktree 政策關閉時排除 `apply-with-worktree` 與 `worktree-merge`）。frontmatter SHALL 依序僅含 `name`、`description`、`license`、`compatibility`、`metadata`（`author`、`version`、`generatedBy`），SHALL NOT 含 `context`、`agent`、`disallowedTools`。內文 SHALL NOT 含 Claude fork 前言（`## Claude fork context`），而 SHALL 於 frontmatter 之後、對話語言段落（或 `## Harness limits`）之前插入 `## oh-my-pi (omp) harness adaptation` 前言，依技能特性映射 oh-my-pi 原生能力：

- 所有技能 SHALL 包含 **User prompts** 條目，將 AskUserQuestion 映射至 `ask` 工具（不可用時以純文字提問並等待回覆）。
- 唯讀技能（`disallow_edit` 為真者）SHALL 包含 **Read-only guarantee** 條目，禁止以 `edit`、`write`、`ast_edit` 修改工作區檔案。
- `analyze`、`drift`、`trace` SHALL 包含 **Isolated read sweep** 條目，指示不改檔執行，必要時透過 `task` 工具搭配唯讀 `scout` 代理。
- `audit`、`review`、`verify`、`quality` SHALL 將子代理派發映射至 `task` 工具（對應內建代理 `security-reviewer`、`reviewer`、`scout` 或唯讀 `task` 代理），且 `review`、`verify`、`quality` SHALL 說明讀取 `subagentModel.<station>.omp` 對應 oh-my-pi 模型角色（`@default`、`@slow`、`@smol`、`@task`、`@plan`）或 `.omp/agents/` 自訂代理。
- `propose` 與 `ingest` SHALL 包含 **Plan mode input** 條目，將 `local://<slug>-plan.md` 視為計畫來源。
- `apply` 與 `apply-with-worktree` SHALL 包含 **Live progress HUD** 條目，允許以 `todo` 工具鏡像進度並維持 `tasks.md` 為唯一真值來源。

技能之間的引用 SHALL 寫成 `/skill:speclink-<name>`（oh-my-pi 的叫用語法），技能檔 SHALL NOT 含 `$speclink-` 或 `/speclink-`；`{{TOOL}}` 的代換值 SHALL 為 `omp`（建立變更與蓋章指令帶 `--agent omp`）；`{{PLAN_DIR}}` 的代換值 SHALL 為空字串。`omp` SHALL NOT 產生、改寫或剝除任何指令檔（`AGENTS.md`、`CLAUDE.md`、`.omp/rules/`），也 SHALL NOT 寫入 `.omp/agents/` 或 `~/.omp/` 使用者層目錄。同時選取 `pi`、`codex` 或 `claude` 與 `omp` 時，各工具 SHALL 各自生成於所屬目錄，互不合併亦不互相略過。工作區 footprint 偵測（`.speclink.yaml` 無 tools 清單時的遺留回退，以及 desktop checkout 預選）SHALL NOT 因 `.omp/` 或 `.omp/skills/` 存在而推斷 `omp`。

#### Scenario: 選取 omp 生成正確的技能檔與前言

- **WHEN** 在空目錄以 `--tools omp` 或 `--tools oh-my-pi` 執行 speclink init，檢視 `.omp/skills/speclink-propose/SKILL.md` 與 `.omp/skills/speclink-analyze/SKILL.md`
- **THEN** 兩檔皆存在；frontmatter 只有 `name`、`description`、`license`、`compatibility`、`metadata` 五個頂層鍵；內文開頭含 `## oh-my-pi (omp) harness adaptation` 與 `ask` 工具映射；`propose` 含 `local://` plan mode 說明與 `--agent omp`；`analyze` 含唯讀禁止 `edit`/`write`/`ast_edit` 與 `scout` 掃描說明；內文技能引用均為 `/skill:speclink-` 形式且不含 `$speclink-`、`/speclink-` 或 `## Claude fork context`

#### Scenario: omp 與原版 pi 並存時各自獨立

- **WHEN** 在 speclink/config.yaml 無 worktree 鍵的專案以 tools 為 `[pi, omp]` 執行 speclink update
- **THEN** `.pi/skills/` 不含 `speclink-analyze` 且不含 `## oh-my-pi (omp) harness adaptation`；`.omp/skills/` 含 `speclink-analyze`（共 17 支技能）且含 `## oh-my-pi (omp) harness adaptation`；兩目錄各自獨立、互不覆寫

#### Scenario: footprint 偵測不推斷 omp

- **WHEN** 專案含 `.claude/` 目錄與 `.omp/rules/team.md`、`.speclink.yaml` 不含 tools 清單，執行 speclink update
- **THEN** 只生成 `.claude/skills/` 下的技能檔，`.omp/skills/` 不被建立，`.omp/rules/team.md` 位元級不變

## MODIFIED Requirements

### Requirement: tools 自訂描述子的接受與驗證

<!-- BEFORE: 內建工具名清單為 claude、codex、copilot、pi、cursor（別名 agents），保留目錄為 .claude/skills、.agents/skills、.github/skills、.pi/skills、.cursor/skills。 -->

.speclink.yaml 的 tools 清單 SHALL 接受兩種元素形式：內建工具名字串（`claude`、`codex`、`copilot`、`pi`、`cursor`、`agy`、`omp`，以及別名 `agents`、`antigravity`、`oh-my-pi`），或自訂描述子物件（欄位：`name` 必填、`skills_dir` 必填、`instructions_file` 選填且不再驅動任何生成、`invocation` 選填且值域為 `cli` 或 `tool-call`、預設 `cli`）。描述子驗證規則：`name` SHALL 為 kebab-case（2 至 50 字）且 SHALL NOT 與內建工具名或別名（`claude`、`codex`、`agents`、`copilot`、`pi`、`cursor`、`agy`、`antigravity`、`omp`、`oh-my-pi`）衝突；`skills_dir` 與（存在時）`instructions_file` SHALL 為專案根相對路徑，正規化後 SHALL NOT 逸出專案根——`instructions_file` 保留驗證僅為遺留 marker 剝除的定位所需。`skills_dir` SHALL 於驗證時削去結尾的 `/`，其後所有消費端（技能生成、足跡記錄與清理、過期探測的路徑回報）SHALL 一律使用削去後的形式。以下兩條拒絕 SHALL 比對詞法正規化後的路徑（丟棄 `.` 段、以 `..` 回退一段，零檔案系統存取），因此等價拼法與字面拼法同樣被拒：正規化後等同專案根本身的 `skills_dir` SHALL 被拒絕；正規化後等同任一內建工具 skills 目錄（`.claude/skills`、`.agents/skills`、`.github/skills`、`.pi/skills`、`.cursor/skills`、`.omp/skills`）的 `skills_dir` SHALL 被拒絕，且這條比對 SHALL 不分大小寫（macOS 與 Windows 的檔案系統上 `.GitHub/skills` 就是 `.github/skills`，`.OMP/skills` 就是 `.omp/skills`）。描述子仍帶 `instructions_file` 欄位時，workspace 檢查 SHALL 輸出一行棄用提示（非錯誤、不影響 exit code）。驗證失敗時指令 SHALL 以非 0 exit code 結束並輸出單行語義化錯誤訊息（指明錯誤欄位與原因）。

#### Scenario: 合法描述子生成對應工具檔

- **WHEN** .speclink.yaml 的 tools 含描述子 name: xyz-harness、skills_dir: .xyz/skills，執行 speclink update
- **THEN** 生成 .xyz/skills/speclink-*/SKILL.md 技能檔，exit code 為 0，且不生成任何指令檔區塊

#### Scenario: 名稱與內建工具衝突被拒

- **WHEN** tools 含描述子 name: claude，執行 speclink update
- **THEN** exit code 非 0，stderr 單行錯誤訊息指明 name 與內建工具名衝突

#### Scenario: 路徑逸出專案根被拒

- **WHEN** tools 含描述子 skills_dir: ../outside/skills，執行 speclink update
- **THEN** exit code 非 0，stderr 單行錯誤訊息指明 skills_dir 逸出專案根

#### Scenario: 殘留 instructions_file 欄位得棄用提示

- **WHEN** tools 含描述子 name: xyz-harness、skills_dir: .xyz/skills、instructions_file: XYZ.md，執行 speclink update
- **THEN** exit code 為 0、技能檔照常生成，stderr 帶一行棄用提示指明 instructions_file 已不生效

#### Scenario: 結尾分隔符於驗證時削去

- **WHEN** tools 含描述子 skills_dir: .xyz/skills/，執行 speclink update 後再執行技能檔過期探測
- **THEN** 技能檔生成於 .xyz/skills/，且探測回報的受管檔路徑以 `.xyz/skills/speclink-` 起頭、不含連續斜線

#### Scenario: 削去後等同專案根的 skills_dir 被拒

- **WHEN** tools 含描述子 skills_dir 為 `/`、`./`、`.` 或 `.xyz/..`（正規化後皆等同專案根），執行 speclink update
- **THEN** exit code 非 0，stderr 單行錯誤訊息指明 skills_dir 等同專案根；專案根不得出現任何 speclink- 目錄

#### Scenario: 與內建工具 skills 目錄相撞被拒

- **WHEN** tools 含 claude 與描述子 skills_dir 為 `.claude/skills`，或其等價拼法 `./.claude/skills`、`.claude/skills/.`、`.claude//skills`、`.xyz/../.claude/skills`，執行 speclink update
- **THEN** exit code 非 0，stderr 單行錯誤訊息指明該目錄屬內建工具；.claude/skills 下既有技能檔逐字元不變

##### Example: skills_dir 的驗證結果

| skills_dir | 結果 |
| --- | --- |
| `.xyz/skills` | 接受，原樣使用 |
| `.xyz/skills/` | 接受，削為 `.xyz/skills` |
| `../outside/skills` | 拒絕——逸出專案根 |
| `/` | 拒絕——正規化後等同專案根 |
| `.xyz/..` | 拒絕——正規化後等同專案根（等價拼法） |
| `.claude/skills` | 拒絕——內建 claude 的 skills 目錄 |
| `./.claude/skills` | 拒絕——正規化後等同內建目錄（等價拼法） |
| `.claude/skills-extra` | 接受——與內建目錄不同 |
| `.github/skills` | 拒絕——內建 copilot 的 skills 目錄 |
| `./.github/skills` | 拒絕——正規化後等同內建目錄（等價拼法） |
| `.github/skills-extra` | 接受——與內建目錄不同 |
| `.GitHub/skills` | 拒絕——不分大小寫等同內建 copilot 目錄 |
| `.pi/skills` | 拒絕——內建 pi 的 skills 目錄 |
| `.xyz/../.pi/skills` | 拒絕——正規化後等同內建目錄（等價拼法） |
| `.pi/agent` | 接受——與內建目錄不同 |
| `.cursor/skills` | 拒絕——內建 cursor 的 skills 目錄 |
| `.cursor//skills` | 拒絕——正規化後等同內建目錄（等價拼法） |
| `.Cursor/skills` | 拒絕——不分大小寫等同內建 cursor 目錄 |
| `.cursor/rules` | 接受——與內建目錄不同 |
| `.omp/skills` | 拒絕——內建 omp 的 skills 目錄 |
| `./.omp/skills/.` | 拒絕——正規化後等同內建目錄（等價拼法） |
| `.OMP/skills` | 拒絕——不分大小寫等同內建 omp 目錄 |
| `.omp/rules` | 接受——與內建目錄不同 |

#### Scenario: 名為 copilot 或指向 Copilot 目錄的既有描述子升級後被拒

- **WHEN** 升級前可用的 .speclink.yaml 的 tools 含描述子 name: copilot、skills_dir: .copilot-skills，或描述子 name: my-copilot、skills_dir: .github/skills，執行 speclink update
- **THEN** exit code 非 0，stderr 單行錯誤訊息分別指明 name 與內建工具名衝突、或 skills_dir 屬內建 copilot 目錄；任何技能檔與 .github/skills 下既有檔案逐字元不變

#### Scenario: 名為 pi、cursor 或指向其目錄的既有描述子升級後被拒

- **WHEN** 升級前可用的 .speclink.yaml 的 tools 含描述子 name: cursor、skills_dir: .cursor/skills，或描述子 name: my-pi、skills_dir: .pi/skills，或描述子 name: pi、skills_dir: .pi-skills，執行 speclink update
- **THEN** exit code 非 0，stderr 單行錯誤訊息分別指明 name 與內建工具名衝突、或 skills_dir 屬內建 pi 目錄；任何技能檔與 .cursor/skills、.pi/skills 下既有檔案逐字元不變

#### Scenario: 名為 omp、oh-my-pi 或指向 .omp/skills 的既有描述子升級後被拒

- **WHEN** 升級前可用的 .speclink.yaml 的 tools 含描述子 name: omp、skills_dir: .omp-custom，或 name: oh-my-pi、skills_dir: .omp-custom，或 name: my-omp、skills_dir: .omp/skills，執行 speclink update
- **THEN** exit code 非 0，stderr 單行錯誤訊息分別指明 name 與內建工具名衝突、或 skills_dir 屬內建 omp 目錄；任何技能檔與 .omp/skills 下既有檔案逐字元不變

---
### Requirement: init 內建 Agent 工具選擇

<!-- BEFORE: --tools 接受 claude、codex、copilot、pi、cursor 五個工具，非互動與未知工具錯誤僅列這五個工具。 -->

所有 `speclink init`（filesystem 與 Remote Store）SHALL 在任何 Workspace 寫入前解析至少一個內建 Agent 工具。顯式 `--tools` SHALL 接受 `claude`、`codex`、`copilot`、`pi`、`cursor`、`agy`、`omp`（含別名 `agents`、`antigravity`、`oh-my-pi`）中的一個，或其中任意多個以逗號分隔（重複名稱合併為一個）並跳過互動選擇；解析後為空或含未知名稱 SHALL 以非零 exit code 失敗，未知名稱的 stderr 單行訊息 SHALL 列出全部內建工具名（含 `omp`）。未提供 `--tools` 且 stdin 為互動終端時，CLI SHALL 於 stderr 呈現互動式工具選擇並涵蓋 `omp`（顯示名 `oh-my-pi`），允許選任意非空組合；未選任何工具 SHALL NOT 開始 init，並 SHALL 要求至少選取一個工具。未提供 `--tools` 且 stdin 非互動終端時，CLI SHALL 在零寫入狀態以非零 exit code 失敗，stderr SHALL 為單行訊息並指出 `--tools`、全部支援工具名（含 `omp`）與逗號分隔的組合寫法，stdout SHALL 為空。init SHALL NOT 將 redirected／piped stdin 當作工具答案，SHALL NOT 新增 stdin payload 或 `--json` 介面。此行為是對早期 footprint 自動偵測的刻意分歧；顯式提供 `--tools` 時成功輸出的格式與 `--no-color` 行為 SHALL 維持既有基線（Generated files 清單內容隨受管集合縮減為技能檔與工作區檔）。

#### Scenario: filesystem init 顯式選擇 Codex

- **WHEN** 在空目錄執行 filesystem init 並顯式提供 `--tools codex`
- **THEN** exit code 為 0，stdout 沿用既有 Initialized 與 Generated files 摘要，`.speclink.yaml` 的 built-in tools 僅含 `codex`，並生成 Codex Skills；不生成 `AGENTS.md`

#### Scenario: Remote init 顯式選擇兩個工具

- **WHEN** 在空目錄執行 Remote Store init 並顯式提供 `--tools claude,codex`、有效 project URL 與 repo
- **THEN** exit code 為 0，`.speclink.yaml` 同時含兩個 built-in tools 與 remote section，生成兩組 Skills，且不存在 `speclink/` 與任何指令檔

#### Scenario: 互動終端選擇 Claude 與 Codex

- **WHEN** 未提供 `--tools` 且 stdin 為互動終端，使用者於互動選擇器勾選 Claude 與 Codex 並確認
- **THEN** 互動介面只寫入 stderr，init 以 claude 與 codex 兩個工具執行，成功摘要寫入 stdout

#### Scenario: 互動終端不得提交空選集

- **WHEN** 未提供 `--tools` 且 stdin 為互動終端，使用者取消全部勾選並嘗試確認
- **THEN** CLI 不建立 `.speclink.yaml`、`speclink/`、`.gitignore` 或 Skills，於 stderr 提示至少選取一個工具並留在選擇器

#### Scenario: 非互動 init 缺少 tools 零寫入失敗

- **WHEN** stdin 為 pipe 或 redirect，執行 init 且未提供 `--tools`
- **THEN** exit code 非 0、stdout 為空，stderr 單行訊息包含 `--tools`、`claude`、`codex`、`copilot`、`pi`、`cursor`、`agy` 與 `omp`，且目標目錄內容逐項不變

#### Scenario: 空或未知的顯式 tools 被拒

- **WHEN** 執行 init 並提供空的 `--tools` 值或含 `vscode` 的值
- **THEN** exit code 非 0，stderr 指出選集為空或 unknown tool（unknown tool 訊息列出 `claude`、`codex`、`copilot`、`pi`、`cursor`、`agy`、`omp`），且任何 Workspace 檔案都未建立或修改

#### Scenario: no-color 不改變工具選擇語意

- **WHEN** 以 `--no-color` 執行互動式 init 並選取任一有效工具
- **THEN** 互動介面與成功輸出不含 ANSI escape sequence，exit code 與檔案效果和有色模式相同

#### Scenario: filesystem init 顯式選擇 Copilot

- **WHEN** 在空目錄執行 filesystem init 並顯式提供 `--tools copilot`
- **THEN** exit code 為 0，stdout 的 Generated files 摘要列出 `copilot`，`.speclink.yaml` 的 built-in tools 僅含 `copilot`，並生成 `.github/skills/` 下的 Copilot Skills；不生成 `.claude/`、`.agents/`、`AGENTS.md`、`CLAUDE.md` 與 `.github/copilot-instructions.md`

#### Scenario: 顯式選擇三個工具

- **WHEN** 在空目錄執行 filesystem init 並顯式提供 `--tools claude,codex,copilot`
- **THEN** exit code 為 0，`.speclink.yaml` 的 built-in tools 依 claude、codex、copilot 記錄三個工具，並生成 `.claude/skills/`、`.agents/skills/`、`.github/skills/` 三組 Skills

#### Scenario: filesystem init 顯式選擇 Pi

- **WHEN** 在空目錄執行 filesystem init 並顯式提供 `--tools pi`
- **THEN** exit code 為 0，stdout 的 Generated files 摘要列出 `pi`，`.speclink.yaml` 的 built-in tools 僅含 `pi`，並生成 `.pi/skills/` 下的 Pi Skills；不生成 `.claude/`、`.agents/`、`.github/`、`.cursor/`、`AGENTS.md` 與 `CLAUDE.md`

#### Scenario: filesystem init 顯式選擇 Cursor

- **WHEN** 在空目錄執行 filesystem init 並顯式提供 `--tools cursor`
- **THEN** exit code 為 0，stdout 的 Generated files 摘要列出 `cursor`，`.speclink.yaml` 的 built-in tools 僅含 `cursor`，並生成 `.cursor/skills/` 下的 Cursor Skills；`.cursor/` 下只有 `skills/` 目錄，不生成 `.cursor/agents/`、`.cursor/rules/`、`.claude/`、`.agents/`、`AGENTS.md` 與 `CLAUDE.md`

#### Scenario: 顯式選擇五個工具

- **WHEN** 在空目錄執行 filesystem init 並顯式提供 `--tools claude,codex,copilot,pi,cursor`
- **THEN** exit code 為 0，`.speclink.yaml` 的 built-in tools 依 claude、codex、copilot、pi、cursor 記錄五個工具，並生成 `.claude/skills/`、`.agents/skills/`、`.github/skills/`、`.pi/skills/`、`.cursor/skills/` 五組 Skills

#### Scenario: filesystem init 顯式選擇 omp 或別名 oh-my-pi

- **WHEN** 在空目錄執行 filesystem init 並顯式提供 `--tools omp` 或 `--tools oh-my-pi`
- **THEN** exit code 為 0，stdout 的 Generated files 摘要列出 `omp`，`.speclink.yaml` 的 built-in tools 僅含 `omp`，並生成 `.omp/skills/` 下的 oh-my-pi Skills（含 `speclink-analyze`）；不生成 `.claude/`、`.agents/`、`.pi/`、`AGENTS.md`、`CLAUDE.md` 與 `.omp/agents/`

##### Example: --tools 值的解析

| `--tools` 值 | 結果 |
| --- | --- |
| `copilot` | 接受，選集為 copilot |
| `Copilot` | 接受，選集為 copilot（大小寫不拘） |
| `claude,copilot` | 接受，選集為 claude、copilot |
| `copilot,copilot` | 接受，選集為 copilot（重複合併） |
| `github-copilot` | 拒絕——unknown tool，訊息列出全部內建工具名 |
| `pi` | 接受，選集為 pi |
| `Cursor` | 接受，選集為 cursor（大小寫不拘） |
| `codex,pi` | 接受，選集為 codex、pi |
| `omp` | 接受，選集為 omp |
| `oh-my-pi` | 接受，選集為 omp（別名正規化） |
| `omp,oh-my-pi` | 接受，選集為 omp（重複合併） |
| `cursor-agent` | 拒絕——unknown tool（不收別名） |
| `vscode` | 拒絕——unknown tool |

---
### Requirement: built-in tools 權威收斂

<!-- BEFORE: 權威收斂與清理集合涵蓋 Claude、Codex、Copilot、Pi、Cursor 五個工具。 -->

Workspace 工具同步 SHALL 將請求中的內建工具集合（涵蓋 `claude`、`codex`、`copilot`、`pi`、`cursor`、`agy`、`omp`）視為 built-in tools 的完整期望狀態。被選取的 built-in SHALL 生成或更新其 `speclink-*` Skills；未選取的 built-in SHALL 移除 Speclink 產生的 Skills。未選取的 `copilot`、`pi`、`cursor`、`omp` 其移除範圍 SHALL 僅限各自 skills 目錄（`.github/skills/`、`.pi/skills/`、`.cursor/skills/`、`.omp/skills/`）下名稱以 `speclink-` 為前綴的目錄（因而變空的 skills 目錄與其所在的 `.github/`、`.pi/`、`.cursor/`、`.omp/` 一併移除；同步前就是空的目錄 SHALL 保留）；`.github/`、`.pi/`、`.cursor/`、`.omp/` 下其他內容 SHALL 位元級不變。任一未選取 built-in 的 skills 目錄路徑上任何一層是 symlink 時（skills 目錄本身，或其上層目錄，例如 `.cursor` 連到 `.claude` 或 `.omp` 連到 `.claude`），同步 SHALL NOT 透過它移除任何目錄，symlink 本身保留。同步 SHALL 在任何寫入之前（含 `.speclink.yaml` 的改寫）檢查 skills 目錄有沒有經 symlink 共用：每個已選取工具（內建工具與自訂描述子）的 skills 目錄順著 symlink 解析後，若和另一個已選取工具的 skills 目錄解析後是同一個目錄，或和一個未選取、路徑上沒有 symlink 的內建工具 skills 目錄（同步會清理它）是同一個目錄，同步 SHALL 以非零 exit code 失敗，輸出一行錯誤指出兩個目錄與所屬工具，且 SHALL NOT 寫入任何檔案；`speclink update --allow-downgrade` SHALL NOT 越過這項檢查。解析後指到其他位置（例如專案外的共用資料夾）的 symlink 不受影響，照常同步。無論選取與否，同步 SHALL 剝除 claude 與 codex 指令檔（`CLAUDE.md`、`AGENTS.md`）中遺留的 `SPECLINK:START..END` 區塊——SHALL 保留區塊外的使用者內容，且指令檔在剝除後全空時才刪除；不含區塊的指令檔 SHALL 位元級不變。`copilot`、`pi`、`cursor`、`omp` 沒有指令檔：同步 SHALL NOT 建立、改寫或刪除 `.github/copilot-instructions.md`，也 SHALL NOT 建立 `.cursor/rules/`、`.cursor/agents/`、`.omp/rules/` 或 `.omp/agents/`。同步 SHALL 保留 `.speclink.yaml` 內的 custom descriptor、unknown tool entry、remote、spec_dir 與其他頂層鍵。相同期望狀態重試 SHALL 收斂到相同檔案結果，不破壞使用者內容。

#### Scenario: Claude 切換為 Codex並保留自訂工具

- **WHEN** `.speclink.yaml` 含 `claude`、一個 custom descriptor、remote section 與未知頂層鍵，`CLAUDE.md` 同時含遺留 Speclink 區塊和使用者文字，然後同步 built-in 選集 `[codex]`
- **THEN** `tools` 保留 custom descriptor 並將 built-in 集合改為僅 `codex`，remote 與未知鍵值不變，Codex Skills 被補齊，Claude Skills 被移除，`CLAUDE.md` 的遺留區塊被剝除且使用者文字仍存在

##### Example: built-in 選集轉換

| 原 built-in | 新選集 | 保留 custom descriptor | 受管結果 |
| --- | --- | --- | --- |
| claude | codex | 是 | 移除 Claude Skills、補齊 Codex Skills |
| claude,codex | claude | 是 | 更新 Claude Skills、移除 Codex Skills |
| codex | claude,codex | 是 | 補齊兩組 Skills |
| claude | claude,copilot | 是 | 更新 Claude Skills、補齊 Copilot Skills |
| claude,copilot | claude | 是 | 更新 Claude Skills、移除 Copilot Skills |
| codex | copilot | 是 | 移除 Codex Skills、補齊 Copilot Skills |
| codex | codex,pi | 是 | 更新 Codex Skills、補齊 Pi Skills |
| codex,pi | codex | 是 | 更新 Codex Skills、移除 Pi Skills |
| claude | claude,cursor | 是 | 更新 Claude Skills、補齊 Cursor Skills |
| claude,cursor | claude | 是 | 更新 Claude Skills、移除 Cursor Skills |
| claude | claude,omp | 是 | 更新 Claude Skills、補齊 omp Skills |
| claude,omp | claude | 是 | 更新 Claude Skills、移除 omp Skills |

#### Scenario: 既有選集缺少產物時自動補齊

- **WHEN** `.speclink.yaml` 的 built-in tools 為 `[codex]`，但任一 Codex Skill 缺少，然後再次同步 `[codex]`
- **THEN** 缺少或過期的技能檔被補齊至正典內容，其他使用者檔案維持不變，且同步成功

#### Scenario: 壞設定在寫入前被拒

- **WHEN** `.speclink.yaml` 無法解析，然後請求同步任一 built-in 選集
- **THEN** 同步以單行解析錯誤失敗，原設定、Skills 與指令檔逐字元不變

#### Scenario: 更新時剝除內建工具的遺留 marker

- **WHEN** 舊版引擎注入過 marker 的專案（`CLAUDE.md` 含 SPECLINK 區塊與使用者段落）以 tools=[claude] 執行新版 speclink update
- **THEN** 技能檔再生為現版內容，`CLAUDE.md` 的區塊被剝除、使用者段落原樣保留，stdout 摘要列出被剝除的檔案

#### Scenario: 取消 Copilot 只移除 speclink 技能目錄

- **WHEN** `.speclink.yaml` 的 built-in tools 為 `[claude, copilot]`，`.github/skills/` 下另有使用者自己的 `my-skill/SKILL.md`，`.github/workflows/ci.yml` 與 `.github/copilot-instructions.md` 存在，然後同步 built-in 選集 `[claude]`
- **THEN** `.github/skills/` 下所有 `speclink-` 前綴目錄被移除，`my-skill/SKILL.md`、`.github/workflows/ci.yml` 與 `.github/copilot-instructions.md` 位元級不變，Claude Skills 被更新

#### Scenario: symlink 的 Copilot 技能目錄不被穿透清除

- **WHEN** `.github/skills` 是指向 `../.claude/skills` 的 symlink，`.speclink.yaml` 的 built-in tools 為 `[claude]`，執行 speclink update
- **THEN** `.claude/skills/` 下的 Claude Skills 全數保留，`.github/skills` symlink 本身保留，清除摘要不列 copilot

#### Scenario: 同步前就空的 Copilot 技能目錄保留

- **WHEN** `.speclink.yaml` 的 built-in tools 為 `[claude]`，`.github/skills/` 是同步前就存在的空目錄，執行 speclink update
- **THEN** `.github/skills/` 仍存在

#### Scenario: 取消 Cursor 只移除 speclink 技能目錄

- **WHEN** `.speclink.yaml` 的 built-in tools 為 `[claude, cursor]`，`.cursor/skills/` 下另有使用者自己的 `my-skill/SKILL.md`，`.cursor/rules/style.mdc` 存在，然後同步 built-in 選集 `[claude]`
- **THEN** `.cursor/skills/` 下所有 `speclink-` 前綴目錄被移除，`my-skill/SKILL.md` 與 `.cursor/rules/style.mdc` 位元級不變，`.cursor/skills/` 因仍有使用者技能而保留，Claude Skills 被更新

#### Scenario: 取消 Pi 後變空的目錄一併移除

- **WHEN** `.speclink.yaml` 的 built-in tools 為 `[codex, pi]`，`.pi/` 下只有 speclink 生成的 `skills/speclink-*` 目錄，然後同步 built-in 選集 `[codex]`
- **THEN** `.pi/skills/` 與 `.pi/` 都不存在，`.agents/skills/` 下的 Codex Skills 被更新

#### Scenario: 取消 omp 只移除 speclink 技能目錄並保留使用者設定

- **WHEN** `.speclink.yaml` 的 built-in tools 為 `[claude, omp]`，`.omp/skills/` 下另有使用者自己的 `my-skill/SKILL.md`，`.omp/rules/team.md` 存在，然後同步 built-in 選集 `[claude]`
- **THEN** `.omp/skills/` 下所有 `speclink-` 前綴目錄被移除，`my-skill/SKILL.md` 與 `.omp/rules/team.md` 位元級不變，`.omp/skills/` 保留，Claude Skills 被更新

#### Scenario: 取消 omp 後變空的目錄一併移除

- **WHEN** `.speclink.yaml` 的 built-in tools 為 `[claude, omp]`，`.omp/` 下只有 speclink 生成的 `skills/speclink-*` 目錄，然後同步 built-in 選集 `[claude]`
- **THEN** `.omp/skills/` 與 `.omp/` 都不存在，`.claude/skills/` 下的 Claude Skills 被更新

#### Scenario: 已勾選工具的 skills 目錄經 symlink 指到另一個工具的目錄時拒絕

- **WHEN** `.speclink.yaml` 的 built-in tools 為 `[claude, cursor]`，`.cursor/skills` 是指向 `../.claude/skills` 的 symlink，執行 speclink update，或執行 `speclink update --allow-downgrade`
- **THEN** exit code 非 0，stderr 為一行錯誤，指出 `.cursor/skills`（cursor）與 `.claude/skills`（claude）是同一個目錄；`.claude/skills/` 下的檔案、`.speclink.yaml` 與 symlink 位元級不變

#### Scenario: 指到未選取工具目錄的 symlink 同樣被拒

- **WHEN** `.claude/skills/` 下有 Claude Skills，`.cursor/skills` 是指向 `../.claude/skills` 的 symlink，同步 built-in 選集 `[cursor]`
- **THEN** 同步以一行錯誤失敗，`.claude/skills/` 下的檔案與 `.speclink.yaml` 位元級不變——不會先寫入 Cursor 版、再把它當成未選取的 claude 清掉

#### Scenario: 指到其他位置的 symlink 照常同步

- **WHEN** `.speclink.yaml` 的 built-in tools 為 `[claude]`，`.claude` 是指向專案外共用資料夾的 symlink，執行 speclink update
- **THEN** exit code 為 0，Claude Skills 寫入該共用資料夾下的 `skills/`

#### Scenario: 取消勾選工具的上層目錄是 symlink 時不清理

- **WHEN** `.speclink.yaml` 的 built-in tools 為 `[claude]`，`.cursor` 是指向 `.claude` 的 symlink（所以 `.cursor/skills` 就是 `.claude/skills`），執行 speclink update
- **THEN** exit code 為 0，`.claude/skills/` 下的 Claude Skills 全數保留，`.cursor` symlink 本身保留，清除摘要不列 cursor

---
### Requirement: Copilot 渲染目標

<!-- BEFORE: 內建工具的渲染差異表列五個工具（claude、codex、copilot、pi、cursor）。 -->

內建工具 copilot 的技能檔 SHALL 生成於專案根的 `.github/skills/speclink-<name>/SKILL.md`。生成集合 SHALL 與 codex 相同：registry 中標記為非 Claude 工具也生成的技能（即除 analyze 以外的技能），worktree 政策關閉時排除兩顆 worktree 技能。frontmatter SHALL 依序僅含 name、description、license、compatibility、metadata（author、version、generatedBy），SHALL NOT 含 context、agent、disallowedTools、allowed-tools、argument-hint、user-invocable、disable-model-invocation。內文 SHALL NOT 含 Claude fork 前言（`## Claude fork context`）。技能之間的引用 SHALL 寫成 `/speclink-<name>`，SHALL NOT 出現 `$speclink-`；`{{TOOL}}` 的代換值 SHALL 為 `copilot`，因此技能指示建立變更與蓋章時帶 `--agent copilot`；`{{PLAN_DIR}}` 的代換值 SHALL 為空字串。除叫用前綴與 `{{TOOL}}` 代換值以外，Copilot 技能內容 SHALL 與同一技能的 codex 生成內容逐字相同。copilot SHALL NOT 產生、改寫或剝除任何指令檔（`.github/copilot-instructions.md`、`AGENTS.md`、`CLAUDE.md`）。工作區 footprint 偵測（`.speclink.yaml` 無 tools 清單時的遺留回退，以及 desktop checkout 的預選）SHALL NOT 因 `.github/` 或 `.github/skills/` 存在而推斷 copilot。

#### Scenario: 選取 Copilot 生成正確的技能檔

- **WHEN** 在空目錄以 `--tools copilot` 執行 speclink init，檢視 `.github/skills/speclink-propose/SKILL.md`
- **THEN** frontmatter 只有 name、description、license、compatibility、metadata 五個頂層鍵，name 為 `speclink-propose`；內文引用其他技能寫成 `/speclink-apply` 這類形式、不含 `$speclink-`、不含 `## Claude fork context`，建立變更的指令帶 `--agent copilot`

##### Example: 內建工具的渲染差異

| 工具 | skills 目錄 | 技能引用寫法 | `--agent` 值 | Claude 專用 frontmatter | 含 analyze |
| --- | --- | --- | --- | --- | --- |
| claude | `.claude/skills` | `/speclink-apply` | `claude` | fork 技能有 context、agent；唯讀技能有 disallowedTools | 是 |
| codex | `.agents/skills` | `$speclink-apply` | `codex` | 無 | 否 |
| copilot | `.github/skills` | `/speclink-apply` | `copilot` | 無 | 否 |
| pi | `.pi/skills` | `/skill:speclink-apply` | `pi` | 無 | 否 |
| cursor | `.cursor/skills` | `/speclink-apply` | `cursor` | 無 | 否 |
| agy | `.agents/skills` | `/speclink-apply` | `agy` | 無（改附 Antigravity harness 前言） | 是 |
| omp | `.omp/skills` | `/skill:speclink-apply` | `omp` | 無（改附 oh-my-pi harness 前言） | 是 |

#### Scenario: Copilot 與 Codex 生成集合相同

- **WHEN** 在 speclink/config.yaml 無 worktree 鍵的專案以 tools 為 `[codex, copilot]` 執行 speclink update
- **THEN** `.github/skills/` 與 `.agents/skills/` 下的 `speclink-` 目錄名稱集合相同，兩者都不含 `speclink-analyze`、`speclink-apply-with-worktree`、`speclink-worktree-merge`

#### Scenario: 選取 Copilot 不產生指令檔

- **WHEN** 在空目錄以 `--tools copilot` 執行 speclink init
- **THEN** 專案根不存在 `AGENTS.md`、`CLAUDE.md`，`.github/` 下只有 `skills/` 目錄，不存在 `.github/copilot-instructions.md`

#### Scenario: footprint 偵測不推斷 Copilot

- **WHEN** 專案含 `.claude/` 目錄與 `.github/workflows/ci.yml`、`.speclink.yaml` 不含 tools 清單，執行 speclink update
- **THEN** 只生成 `.claude/skills/` 下的技能檔，`.github/skills/` 不被建立，`.github/workflows/ci.yml` 位元級不變
