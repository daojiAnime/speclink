## MODIFIED Requirements

### Requirement: workflow-config set 政策欄位寫入

<!-- BEFORE: locale 僅接受 tw、ja、en，spec_locale 僅接受 tw、ja、en、auto。 -->

CLI SHALL 提供 speclink workflow-config set <key> <value>（旗標 --dry-run 與 --no-color，無 stdin），寫入政策欄位：key SHALL 限 locale、spec_locale、tdd、audit、worktree 五者，其他 key SHALL 以非零 exit code 拒絕且無檔案效果；tdd、audit 與 worktree 的 value SHALL 僅接受 true 或 false，其他值 SHALL 以非零 exit code 拒絕。locale 的 value SHALL 僅接受語系代碼 cn、tw、ja、en，spec_locale 的 value SHALL 僅接受 cn、tw、ja、en、auto；比對 SHALL 大小寫敏感（TW、CN、Auto 均非法）；空字串（移除鍵、回到未設定＝預設）SHALL 仍合法；其他值——含顯示名稱字串（如「繁體中文」「简体中文」）或 BCP 47 寫法（如 `zh-CN`）——SHALL 以非零 exit code 拒絕，stderr SHALL 指出欄位名、收到的值與合法代碼集合（含 `cn`），且無任何檔案效果；帶 --dry-run 時非法值 SHALL 同樣拒絕且 SHALL NOT 印出 diff。

set 的 CLI help 文字 SHALL 與實際接受的鍵集合一致：speclink workflow-config set --help 的子指令說明所列的政策鍵 SHALL 為前述五鍵，且逐字等同未知 key 錯誤訊息所列的鍵集合；<value> 參數說明 SHALL 指出僅接受 true 或 false 的三個鍵（tdd、audit、worktree）。speclink workflow-config --help 子指令一覽中 set 一列的說明 SHALL 與 set --help 的子指令說明同源同字面。help 輸出無 --json 面，--no-color 亦 SHALL NOT 改變其內容。此為刻意變更：`locale` 與 `spec_locale` 合法代碼集合與非法值錯誤訊息納入 `cn`，其餘成功訊息、diff 輸出、輸出去向與 exit code SHALL 維持既有位元級輸出。

寫入 SHALL 為文字層 read-modify-write：僅變動目標鍵所在的行或區塊，其餘行——含註解行、空行、未知頂層鍵與 schema 鍵下方的使用者自加內容——SHALL 逐位元保留（模板註解不再喪失）。tdd、audit 或 worktree 設為 false、locale 或 spec_locale 設為空字串時 SHALL 移除該鍵行（維持未設定＝預設語意），SHALL NOT 一併刪除其上方的註解行。新增原先不存在的政策鍵時 SHALL 依正典序（locale、spec_locale、tdd、audit、worktree）以連續區塊插於 schema 鍵行之後，區塊與前後內容行之間 SHALL 各恰一空行（相鄰處原已有空行時不重複補）；schema 鍵不存在時 SHALL 插於檔案最頂端。既有政策鍵 SHALL 原位改值、SHALL NOT 移動位置。寫入結果 SHALL 於落檔前重新解析並與目標狀態逐鍵等值比對，不等值 SHALL fail-closed 拒絕寫入且原檔逐位元不變。

帶 --dry-run 時 SHALL 於 stdout 印出變更前後的 unified diff、SHALL NOT 寫入任何檔案、exit code 0；無變更時 diff 為空。fs 模式寫入 speclink/config.yaml；remote 模式 SHALL 先經連線讀取 server 端 config 現行內容與版本，套用同一改寫、寫回時附帶讀得的版本——server 端版本已前進（他人並行改寫）時 SHALL 以非零 exit code 失敗、stderr 提示重新執行，SHALL NOT 覆蓋他人寫入；版本識別 SHALL NOT 出現在指令介面。連線離線或認證失效時 SHALL 以非零 exit code 失敗並輸出語義化訊息，SHALL NOT 暫存或排隊寫入。config 文件無法解析時 SHALL fail-closed 拒絕寫入（重寫壞檔會毀掉使用者內容）。成功 exit code 0、stdout 單行成功訊息。

#### Scenario: 設定 locale 保留其他鍵

- **WHEN** speclink/config.yaml 含 schema、tdd: true 與 context，執行 speclink workflow-config set locale cn
- **THEN** exit code 0；config.yaml 的 locale 為 cn，schema、tdd、context 的值不變

#### Scenario: 缺鍵插於 schema 之下且空行區隔

- **WHEN** config.yaml 依序含 schema、context、rules 而無任何政策鍵，執行 speclink workflow-config set locale tw
- **THEN** exit code 0；locale: tw 行位於 schema 鍵行之後、context 之前，與前後內容行之間各恰一空行；context 與 rules 的所有行逐位元不變

#### Scenario: 註解與空行逐位元保留

- **WHEN** config.yaml 含模板註解與使用者自加的註解行，執行任一合法的 set 寫入
- **THEN** exit code 0；除目標鍵所在行與插入區塊外，檔案所有行（含全部註解與空行）逐位元不變

#### Scenario: 檔尾既有鍵原位改值不搬家

- **WHEN** config.yaml 的 locale 鍵位於檔案最尾（rules 之後），執行 speclink workflow-config set locale ja
- **THEN** exit code 0；locale 行仍在檔尾原位、值改為 ja，其餘行逐位元不變

#### Scenario: schema 缺席時插於檔案最頂端

- **WHEN** config.yaml 無 schema 鍵且無政策鍵，執行 speclink workflow-config set tdd true
- **THEN** exit code 0；tdd: true 位於檔案最頂端，與後續內容之間恰一空行

#### Scenario: 內部改寫驗證失敗拒絕寫入

- **WHEN** 文字層手術產出的結果重新解析後與目標狀態不等值（防呆觸發）
- **THEN** exit code 非 0，stderr 單行錯誤指明內部改寫驗證失敗；speclink/config.yaml 逐位元不變

#### Scenario: 未知 key 拒絕

- **WHEN** 執行 speclink workflow-config set theme dark
- **THEN** exit code 非 0，stderr 指出 key 限 locale、spec_locale、tdd、audit、worktree；speclink/config.yaml 逐位元不變

#### Scenario: set --help 列出全部政策鍵

- **WHEN** 執行 speclink workflow-config set --help
- **THEN** exit code 0；stdout 的子指令說明所列政策鍵為 locale、spec_locale、tdd、audit、worktree，與未知 key 錯誤訊息所列的鍵集合逐字相同

#### Scenario: set --help 標明布林鍵的合法值

- **WHEN** 執行 speclink workflow-config set --help
- **THEN** exit code 0；stdout 的 <value> 參數說明指出 tdd、audit 與 worktree 僅接受 true 或 false

#### Scenario: 父層 help 的 set 說明同源

- **WHEN** 執行 speclink workflow-config --help
- **THEN** exit code 0；子指令一覽中 set 一列的說明與 speclink workflow-config set --help 的子指令說明字面相同（同樣列出五個政策鍵）

#### Scenario: 非法布林值拒絕

- **WHEN** 執行 speclink workflow-config set tdd yes
- **THEN** exit code 非 0，stderr 指出 tdd 僅接受 true 或 false；無任何檔案效果

#### Scenario: 非法 locale 值拒絕

- **WHEN** 執行 speclink workflow-config set locale 繁體中文
- **THEN** exit code 非 0，stderr 指出 locale 欄位、收到的值「繁體中文」與合法代碼 cn、tw、ja、en；speclink/config.yaml 逐位元不變

#### Scenario: 非法值帶 dry-run 同樣拒絕

- **WHEN** 執行 speclink workflow-config set spec_locale 繁體中文 --dry-run
- **THEN** exit code 非 0，stdout 無 diff 輸出，stderr 列出 spec_locale 的合法代碼（cn、tw、ja、en、auto）；無任何檔案效果

##### Example: locale 值域判定

| key | value | 結果 |
| --- | ----- | ---- |
| locale | tw | 成功 |
| locale | cn | 成功 |
| locale | 空字串 | 成功（移除鍵） |
| locale | 繁體中文 | 拒絕 |
| locale | CN | 拒絕（大小寫敏感） |
| spec_locale | cn | 成功 |
| spec_locale | auto | 成功 |
| spec_locale | zh-CN | 拒絕 |

#### Scenario: 設 false 移除鍵

- **WHEN** speclink/config.yaml 含 audit: true，執行 speclink workflow-config set audit false
- **THEN** exit code 0；config.yaml 不含 audit 鍵（未設定＝預設關閉），該鍵上方的註解行仍在

#### Scenario: dry-run 印 diff 不落檔

- **WHEN** 執行 speclink workflow-config set locale ja --dry-run
- **THEN** exit code 0；stdout 為 unified diff 且僅含 locale 相關行的變更（不含其他行的重排）；speclink/config.yaml 逐位元不變

#### Scenario: remote 版本衝突提示重跑

- **WHEN** 於 remote 模式執行 speclink workflow-config set tdd true，且 server 端 config 在讀取後、寫回前已被他人改寫
- **THEN** exit code 非 0，stderr 說明設定已被他人更新、請重新執行；server 端內容維持他人寫入的版本

#### Scenario: 壞 config 拒絕寫入

- **WHEN** speclink/config.yaml 含 YAML 語法錯誤，執行 speclink workflow-config set locale tw
- **THEN** exit code 非 0，stderr 指出該檔無法解析、寫入已拒絕；檔案逐位元不變

---
### Requirement: instructions 對 specs 產出物回報解析後的規格語言

<!-- BEFORE: 僅映射 tw、ja、en；中文模糊用語提醒僅於 tw 與 zh* 觸發且僅列繁體字形。 -->

speclink instructions specs --change <名> --json 的 payload 的 `locale` 欄位（字串）SHALL 是規格語言的顯示名稱。規格語言 SHALL 由 spec_locale 的有效值決定，有效值沿用工作流政策的三層解析（SPECLINK_SPEC_LOCALE ＞ speclink/config.yaml ＞ 內建預設）：

- spec_locale 是 `tw` 時 SHALL 為 `Traditional Chinese (繁體中文)`，是 `cn` 時 SHALL 為 `Simplified Chinese (简体中文)`，是 `ja` 時 SHALL 為 `Japanese (日本語)`，是 `en` 時 SHALL 為 `English`。
- spec_locale 是 `auto` 時 SHALL 為 locale 有效值的顯示名稱；locale 未設定時 SHALL 為 `Simplified Chinese (简体中文)`。
- spec_locale 未設定時 SHALL 為 `Simplified Chinese (简体中文)`，不論 locale 的值（顯式設定 `spec_locale: en` 才回報 `English`）。

其他產出物（proposal、design、tasks）與 speclink instructions apply 的 `locale` SHALL 為 locale 有效值的顯示名稱（`cn` 與未設定均映射為 `Simplified Chinese (简体中文)`，顯式 `en` 映射為 `English`）。payload 的欄位名稱與型別 SHALL 不變。

同一份 payload 的 `instruction` SHALL 以一段寫明規格語言的文字結尾，規格語言是 English 時也寫：這一段 SHALL 含 `Write spec prose in <顯示名稱>`，`<顯示名稱>` 與同一份 payload 的 `locale` 相同；SHALL 說明結構標記（`## ADDED/MODIFIED/REMOVED/RENAMED Requirements`、`### Requirement:`、`#### Scenario:`、`- **WHEN**`／`- **THEN**`）與規範關鍵字（SHALL、MUST、SHALL NOT、MUST NOT）維持英文。規格語言是 `tw` 時，這一段 SHALL 另含繁體中文模糊用語（`應該、可能、也許、或許、大概、考慮、盡量、待定`）會被 analyzer 標記的提醒；規格語言是 `cn`、未設定（預設 `cn`）或小寫 `zh-cn`、`zh-sg`、`zh-hans*` 時，這一段 SHALL 另含简体中文模糊用語（`应该、可能、也许、或许、大概、考虑、尽量、待定`）會被 analyzer 標記的提醒；規格語言是 `ja` 或 English 時 SHALL NOT 含這項提醒。`instruction` SHALL NOT 含 `config.yaml`、`English by default` 與 `spec_locale` 這三段文字。

同一段語言說明 SHALL 寫明需求與情境名稱的規則，四種規格語言（`cn`、`tw`、`ja`、English）都寫，位置在結構標記與規範關鍵字的說明之後、中文模糊用語的提醒之前：

- 名稱是內文，跟著規格語言寫：這一段 SHALL 含 `Requirement and scenario names are prose too: write them in <顯示名稱>`，`<顯示名稱>` 與同一份 payload 的 `locale` 相同。
- 指向正式規格的名稱照正式規格的原文，不論原文是哪一種語言：這一段 SHALL 含 `exactly as the canonical spec writes it`，並 SHALL 逐一點名四種名稱——`## MODIFIED Requirements` 與 `## REMOVED Requirements` 底下的需求名稱、RENAMED 項目的 `FROM:` 名稱、MODIFIED 區塊沿用的正式規格情境名稱、`<!-- REMOVED-SCENARIO: … -->` 宣告裡的名稱。
- 這一段 SHALL 說明改掉這些名稱會讓變更的驗證失敗。

其他產出物（proposal、design、tasks）與 speclink instructions apply 的 `instruction` SHALL NOT 含名稱的規則。schema 的 specs 產出物沒有 instruction 時，這一段 SHALL 是整個 `instruction`。

人眼輸出（不帶 --json）SHALL 維持不顯示 `locale` 欄位，它的 Instruction 區段 SHALL 是同一段 `instruction` 文字（因此以同一段語言說明結尾）；`--no-color` 下文字相同、不含色碼。本機 repo、連到遠端的 CLI（payload 由 Server 以同一個入口計算，CLI 原樣輸出）與宿主套件的 dispatch SHALL 對相同的設定得到相同的 `locale` 與 `instruction`。speclink/config.yaml 無法解析時 SHALL 照「工作流政策的正典歸屬與三層解析順序」以非零 exit code 失敗，SHALL NOT 以預設語言繼續；找不到變更時 SHALL 以非零 exit code 失敗、stderr 指出找不到該變更，stdout 不輸出 payload。

此為刻意變更：新增 `cn` 映射為 `Simplified Chinese (简体中文)`、將未設定預設改為 `Simplified Chinese (简体中文)` 並附简体中文模糊用語提醒；顯式設定 `tw`、`ja`、`en` 的輸出維持不變。

#### Scenario: 沒有設定 spec_locale 時規格語言是 English

- **WHEN** speclink/config.yaml 設定 locale: tw 且未設定 spec_locale，執行 speclink instructions specs --change 某 change --json
- **THEN** exit code 為 0；payload 的 locale 為 Simplified Chinese (简体中文)；instruction 的最後一段含 Write spec prose in Simplified Chinese (简体中文) 與简体中文模糊用語提醒；顯式設定 spec_locale: en 時 payload 的 locale 為 English 且不含中文模糊用語提醒

#### Scenario: auto 跟著 locale

- **WHEN** speclink/config.yaml 設定 locale: tw 與 spec_locale: auto，執行 speclink instructions specs --change 某 change --json
- **THEN** payload 的 locale 為 Traditional Chinese (繁體中文)；instruction 的最後一段含 Write spec prose in Traditional Chinese (繁體中文) 與中文模糊用語的提醒；instruction 不含 spec_locale 這段文字

#### Scenario: 簡體中文規格語言與提醒

- **WHEN** speclink/config.yaml 設定 locale: cn 與 spec_locale: auto，執行 speclink instructions specs --change 某 change --json
- **THEN** exit code 為 0；payload 的 locale 為 Simplified Chinese (简体中文)；instruction 的最後一段含 Write spec prose in Simplified Chinese (简体中文)、Requirement and scenario names are prose too: write them in Simplified Chinese (简体中文)，並以简体中文模糊用語（应该、可能、也许、或许、大概、考虑、尽量、待定）提醒收尾

#### Scenario: 規格語言與 locale 不同時各自回報

- **WHEN** speclink/config.yaml 設定 locale: tw 與 spec_locale: ja，分別執行 speclink instructions specs 與 speclink instructions proposal（同一個 change，帶 --json）
- **THEN** specs 的 locale 為 Japanese (日本語)，proposal 的 locale 為 Traditional Chinese (繁體中文)

#### Scenario: 環境變數覆寫規格語言

- **WHEN** speclink/config.yaml 設定 spec_locale: en，設定環境變數 SPECLINK_SPEC_LOCALE=ja，執行 speclink instructions specs --change 某 change --json
- **THEN** payload 的 locale 為 Japanese (日本語)；instruction 的最後一段含 Write spec prose in Japanese (日本語)

#### Scenario: 指示不要求讀設定檔

- **WHEN** 以任一合法的 locale 與 spec_locale 組合執行 speclink instructions specs --change 某 change --json
- **THEN** instruction 不含 config.yaml，也不含 English by default

#### Scenario: 經宿主套件得到相同的回應

- **WHEN** 宿主套件的範圍設定 locale: tw、未設定 spec_locale，以 dispatch 執行 instructions specs --change 某 change --json
- **THEN** 回傳物件的 locale 為 Simplified Chinese (简体中文)，instruction 的最後一段含 Write spec prose in Simplified Chinese (简体中文) 與简体中文模糊用語提醒，與本機 CLI 對相同設定的 payload 相同

#### Scenario: 設定檔無法解析時不回報語言

- **WHEN** speclink/config.yaml 含 YAML 語法錯誤，執行 speclink instructions specs --change 某 change --json
- **THEN** exit code 非 0，stderr 指出該檔的路徑與解析原因，stdout 不輸出 payload

#### Scenario: 語言說明寫明名稱的規則

- **WHEN** speclink/config.yaml 設定 locale: tw 與 spec_locale: tw，執行 speclink instructions specs --change 某 change --json
- **THEN** exit code 為 0；instruction 的最後一段含 Requirement and scenario names are prose too: write them in Traditional Chinese (繁體中文)、exactly as the canonical spec writes it、## MODIFIED Requirements、## REMOVED Requirements、FROM: 與 REMOVED-SCENARIO；這些文字都出現在中文模糊用語的提醒之前

#### Scenario: 規格語言是 English 或日文時也寫名稱的規則

- **WHEN** speclink/config.yaml 設定 spec_locale: en，以及設定 spec_locale: ja，分別執行 speclink instructions specs --change 某 change --json
- **THEN** 兩次的 instruction 最後一段都含 exactly as the canonical spec writes it 與 REMOVED-SCENARIO；en 時含 write them in English，ja 時含 write them in Japanese (日本語)；兩次都不含中文模糊用語的提醒

#### Scenario: 其他產出物的指示沒有名稱的規則

- **WHEN** 以 spec_locale: tw 分別執行 speclink instructions proposal、design、tasks 與 apply（同一個 change，帶 --json）
- **THEN** 四份 payload 的 instruction 都不含 Requirement and scenario names are prose

#### Scenario: 人眼輸出帶同一段名稱規則

- **WHEN** 以 spec_locale: tw 執行 speclink instructions specs --change 某 change（不帶 --json），再加上 --no-color 執行一次
- **THEN** 兩次的 Instruction 區段都以含 Write spec prose in Traditional Chinese (繁體中文) 與 exactly as the canonical spec writes it 的同一段文字結尾；加 --no-color 的輸出不含色碼

##### Example: 規格語言的解析

| locale | spec_locale | SPECLINK_SPEC_LOCALE | specs 的 locale | proposal 的 locale |
| ------ | ----------- | -------------------- | --------------- | ------------------ |
| tw     | 未設定      | 未設定               | Simplified Chinese (简体中文) | Traditional Chinese (繁體中文) |
| cn     | 未設定      | 未設定               | Simplified Chinese (简体中文) | Simplified Chinese (简体中文) |
| cn     | auto        | 未設定               | Simplified Chinese (简体中文) | Simplified Chinese (简体中文) |
| tw     | en          | 未設定               | English         | Traditional Chinese (繁體中文) |
| tw     | auto        | 未設定               | Traditional Chinese (繁體中文) | Traditional Chinese (繁體中文) |
| tw     | ja          | 未設定               | Japanese (日本語) | Traditional Chinese (繁體中文) |
| 未設定 | auto        | 未設定               | Simplified Chinese (简体中文) | Simplified Chinese (简体中文) |
| 未設定 | cn          | 未設定               | Simplified Chinese (简体中文) | Simplified Chinese (简体中文) |
| 未設定 | tw          | 未設定               | Traditional Chinese (繁體中文) | Simplified Chinese (简体中文) |
| tw     | en          | cn                   | Simplified Chinese (简体中文) | Traditional Chinese (繁體中文) |
| tw     | en          | ja                   | Japanese (日本語) | Traditional Chinese (繁體中文) |
