## MODIFIED Requirements

### Requirement: workflow-config show 動詞

<!-- BEFORE: builtinSubagentModels 僅包含 claude（程式碼現況另含 agy），不含 omp。 -->

CLI SHALL 提供 speclink workflow-config show（旗標 `--json` 與 `--no-color`，無 stdin），顯示 speclink/config.yaml 的正典內容：schema（專案預設 schema 名稱——未設定時標示為未設定與其預設 spec-driven）、政策五欄（locale、spec_locale、tdd、audit、worktree——未設定的欄位標示為未設定與其預設語意）、context（有無與行數）、rules（各 artifact 節的條數）與 review（預設清單模式與三個嚴重度清單的條數）。show SHALL 顯示正典值，SHALL NOT 應用環境變數的覆寫（有效值的三層解析屬 instructions payload 職責）。帶 `--json` 時 SHALL 輸出 camelCase payload：schema（字串，未設定為 null）、locale、specLocale（未設定為 null）、tdd、audit、worktree（布林）、context（字串或 null）、rules（artifact id 對規則字串陣列的物件）、review（物件：baseline 為字串 "extend" 或 "replace"，critical、warning、suggestion 為規則字串陣列；檔案未寫 review 區塊時為 baseline "extend" 與三個空陣列）、subagentModel（物件：review、verify 為 AI 工具名稱對模型名稱字串的物件，choices 為 AI 工具名稱對模型名稱字串陣列的物件；檔案未寫 subagent_model 區塊時為三個空物件）與 builtinSubagentModels（引擎內建的子代理模型清單：AI 工具名稱對模型名稱字串陣列的物件，包含 `"claude": ["sonnet", "opus", "haiku", "fable"]`、`"agy": ["inherit", "pro", "flash", "flash_lite"]` 與 `"omp": ["@default", "@slow", "@smol", "@task", "@plan"]`，不隨 config 內容改變）。人眼輸出 SHALL 於標題空行之後、locale 行之前，以與其他行相同的標籤欄寬印出 schema 行：值為 config 的 schema 名稱，未設定時為 unset (spec-driven)。人眼輸出 SHALL 於 rules 行之後、以與其他行相同的標籤欄寬印出 review 行：baseline 為 extend（含未設定）且三個清單皆空時值為 none，否則值為 `<baseline>, critical <n>, warning <n>, suggestion <n>`。fs 模式讀取 speclink/config.yaml；remote 模式 SHALL 經既有連線讀取 server 端 config 文件，人眼與 `--json` 輸出形狀與 fs 模式一致；離線或認證失效時 SHALL 沿用既有 remote 錯誤路徑以非零 exit code 結束且不輸出 payload。config 文件無法解析時 SHALL 沿用既有 fail-closed 行為：非零 exit code、stderr 指出檔案與解析原因。本機模式與 SDK 的 dispatch SHALL 經引擎的命令執行層執行本動詞，SHALL NOT 自行讀取檔案或儲存；remote 模式的 CLI 經連線取得 server 的文件後，SHALL 以同一個引擎解析與同一份輸出呈現。成功 exit code 0、輸出至 stdout；`--no-color` 下無 ANSI 色彩。本次修訂為刻意變更：`--json` 的 `builtinSubagentModels` 新增 `"omp"` 鍵（值為 `["@default", "@slow", "@smol", "@task", "@plan"]`）；人眼輸出與其餘 `--json` 欄位 SHALL 維持既有位元級輸出。

#### Scenario: fs 模式顯示正典值

- **WHEN** speclink/config.yaml 含 locale: tw、tdd: true 與 context，執行 speclink workflow-config show
- **THEN** stdout 顯示 locale 為 tw、tdd 開啟、audit 未設定（預設關閉）、worktree 未設定（預設關閉）、context 存在；exit code 0

#### Scenario: --json payload 形狀

- **WHEN** 執行 speclink workflow-config show --json
- **THEN** stdout 為 JSON，欄位 schema、locale、specLocale、tdd、audit、worktree、context、rules、review、subagentModel、builtinSubagentModels 一律 camelCase；未設定的 schema 為 null、未設定的 specLocale 為 null、未設定的 tdd 為 false、未設定的 worktree 為 false、未寫 review 區塊時 review 為 baseline "extend" 與 critical、warning、suggestion 三個空陣列、未寫 subagent_model 區塊時 subagentModel 為 `{"review":{},"verify":{},"choices":{}}`、builtinSubagentModels 含 `"claude": ["sonnet", "opus", "haiku", "fable"]`、`"agy": ["inherit", "pro", "flash", "flash_lite"]` 與 `"omp": ["@default", "@slow", "@smol", "@task", "@plan"]`

#### Scenario: show 不應用環境變數覆寫

- **WHEN** 設定 SPECLINK_TDD=false 且 speclink/config.yaml 含 tdd: true，執行 speclink workflow-config show --json
- **THEN** payload 的 tdd 為 true（正典值，非解析後有效值）

#### Scenario: remote 模式輸出形狀一致

- **WHEN** 於 remote 綁定的 workspace 執行 speclink workflow-config show --json
- **THEN** stdout 的 JSON 欄位名與 fs 模式完全一致（含 `builtinSubagentModels.omp`），值來自 server 端 config 文件

#### Scenario: 壞 config fail-closed

- **WHEN** speclink/config.yaml 含 YAML 語法錯誤，執行 speclink workflow-config show
- **THEN** exit code 非 0，stderr 指出該檔與解析原因，stdout 無 payload

#### Scenario: 人眼輸出的 review 行

- **WHEN** 執行 speclink workflow-config show --no-color
- **THEN** stdout 於 rules 行之後恰有一行 review，標籤欄寬與 rules 行相同，值依 review 區塊內容而定；其餘各行與本次修訂前逐位元一致

##### Example: review 行的值

| config.yaml 的 review 區塊 | review 行的值 |
| -------------------------- | ------------- |
| 未寫 review | none |
| critical 兩條、warning 一條、suggestion 一條，未寫 baseline | extend, critical 2, warning 1, suggestion 1 |
| baseline: replace、三個清單皆空 | replace, critical 0, warning 0, suggestion 0 |

#### Scenario: --json 帶出 review 內容

- **WHEN** speclink/config.yaml 含 review 區塊 baseline: replace 與 critical 一條「所有 SQL 一律用參數化查詢」，執行 speclink workflow-config show --json
- **THEN** payload 的 review 為 `{"baseline":"replace","critical":["所有 SQL 一律用參數化查詢"],"warning":[],"suggestion":[]}`；exit code 0

#### Scenario: --json 帶出子代理模型設定

- **WHEN** speclink/config.yaml 含 subagent_model 區塊，review 為 claude: opus 與 codex: gpt-6.1-sol、verify 為 claude: sonnet、choices 為 codex 兩條 gpt-6.1-sol 與 gpt-6-astra，執行 speclink workflow-config show --json
- **THEN** exit code 0；payload 的 subagentModel 為 `{"review":{"claude":"opus","codex":"gpt-6.1-sol"},"verify":{"claude":"sonnet"},"choices":{"codex":["gpt-6.1-sol","gpt-6-astra"]}}`，builtinSubagentModels 含 `"claude"`、`"agy"` 與 `"omp"` 的內建清單

#### Scenario: --json 正確回傳 omp 角色別名設定

- **WHEN** speclink/config.yaml 的 subagent_model 區塊設 review.omp 為 `"@slow"`、verify.omp 為 `"@smol"`，執行 speclink workflow-config show --json
- **THEN** exit code 0；payload 的 `subagentModel.review.omp` 為 `"@slow"`、`subagentModel.verify.omp` 為 `"@smol"`，且 `builtinSubagentModels.omp` 為 `["@default", "@slow", "@smol", "@task", "@plan"]`

#### Scenario: 人眼輸出不顯示子代理模型設定

- **WHEN** speclink/config.yaml 含上一個 Scenario 的 subagent_model 區塊，執行 speclink workflow-config show --no-color
- **THEN** exit code 0；stdout 與同一份 config 刪掉 subagent_model 區塊後的輸出逐位元一致

#### Scenario: 顯示專案預設 schema

- **WHEN** 執行 speclink workflow-config show --no-color 與 speclink workflow-config show --json
- **THEN** 人眼輸出於 locale 行之前恰有一行 schema，標籤欄寬與 locale 行相同；--json 的 schema 欄位值依 config 的 schema 鍵而定；exit code 0

##### Example: schema 的值

| config.yaml 的 schema 鍵 | 人眼 schema 行的值 | --json 的 schema |
| ------------------------ | ------------------ | ---------------- |
| 未寫 schema | unset (spec-driven) | null |
| schema: spec-driven | spec-driven | "spec-driven" |
| schema: my-flow | my-flow | "my-flow" |

#### Scenario: show 經命令執行層

- **WHEN** 分別執行 speclink workflow-config show --json 與 engine.dispatch(['workflow-config', 'show'])
- **THEN** dispatch 解析出的物件與 CLI 的 stdout 經 JSON 解析後的值相等
