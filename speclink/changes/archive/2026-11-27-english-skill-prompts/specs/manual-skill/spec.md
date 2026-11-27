## MODIFIED Requirements

### Requirement: 動線由引數選擇

<!-- BEFORE: 引數表格以繁體中文「導覽」或 `tour` 分流導覽模式，範圍提示舉例為「全部重生」。 -->

技能 SHALL 依呼叫引數分流：無引數 SHALL 走生成模式；引數含 `tour` SHALL 走導覽模式；其他引數 SHALL 視為生成模式的範圍提示（例如單一分區、單頁或 `regenerate all`）並於摘要中覆述所理解的範圍。此為刻意變更：原稿分流條件與舉例統一改為英文，與 `skill-authoring` 的英文原稿規範一致並同批更新 golden 快照。

#### Scenario: 無引數走生成

- **WHEN** 使用者呼叫 /speclink-manual 不帶引數
- **THEN** 技能進入生成模式，結束時輸出生成摘要

#### Scenario: 導覽引數走導覽

- **WHEN** 使用者呼叫 /speclink-manual tour
- **THEN** 技能進入導覽模式，不寫任何檔案

---
### Requirement: 生成模式的輸出與報告

<!-- BEFORE: 摘要範本以繁體中文 `## 手冊生成摘要` 與繁體中文欄位名撰寫，YAML 與錨點範例採繁體中文。 -->

生成模式 SHALL 依 manual-pages 契約寫頁（頁尾來源行 `**Sources**:`、`about.md` 標題 `About this manual`、無矛盾標記 `None found`、無能力標記 `No user-facing capabilities yet`），每頁 frontmatter 的 generated SHALL 為生成當下帶時區偏移量的 RFC 3339 時戳（秒級，例 `2026-09-05T23:31:00+08:00`），技能檔 SHALL 給 agent 一條取得該時戳的建議指令；已有手冊時預設 SHALL 只重生過期頁並為未入冊能力新增頁，使用者明示要求時方全部重新產生。過期判定 SHALL 與 manual-pages 契約「過期判定基準」同基準，含 sources 項的 Requirement 錨定：帶錨定的項只看該 Requirement 段落的 @trace updated，錨定找不到即過期。新頁與整頁重寫的頁，其 sources 每項 SHALL 依取材範圍寫出：該頁只取材該 capability 的部分 Requirement 時，每條寫一項帶錨定的 `"<capability>#<Requirement name>"`（以雙引號包住）；取材該 capability 全部 Requirement 時寫不帶錨定的名稱。被判過期的頁重生後（含重新推導的 sources）除 generated 行外與磁碟內容逐位元相同時，SHALL 只把 generated 換成本次時戳、其餘位元不動，並在摘要的 `Stamp-only` 列計入該頁；內文或 sources 不同時整頁重寫並計入 `Regenerated`——錨定找不到而過期的頁 SHALL NOT 走只換時戳。index.md 與 about.md SHALL 在任一頁新增、重生或只換時戳時隨之重生。技能檔 SHALL 把 sources 錨定的語意集中寫成一段，其他提及 sources 的規則引用該段。結束時 SHALL 依英文範本 `## Manual Generation Summary` 輸出摘要：Added、Regenerated、Stamp-only、Untouched 頁數；Possibly stale 頁清單；Unlisted capabilities 清單；Contradictions recorded on about page 數量；Skipped or missing sources 清單。無過期頁且無未入冊能力時 SHALL 零檔案寫入並如實回報。摘要末尾 SHALL 建議以一般 git commit 收尾手冊異動——僅建議、SHALL NOT 代跑。此為刻意變更：摘要範本與範例全面改為英文並同批更新 golden 快照，語意與過期判定基準不變。

#### Scenario: 首次生成完整手冊

- **WHEN** 工作區無 speclink/manual/ 而執行生成模式
- **THEN** 產出含 index.md 與 about.md 的完整手冊，每頁 generated 為 RFC 3339 時戳，只取材部分 Requirement 的頁其 sources 帶錨定，摘要列出 Added 頁數且 Regenerated、Stamp-only 與 Untouched 為零

#### Scenario: 二次只重生過期頁

- **WHEN** 手冊已存在且兩頁過期、一個能力未入冊
- **THEN** 僅該兩頁被重寫（generated 換成本次時戳）、新增一頁，其餘頁逐位元不變，摘要列出過期頁與新入冊能力

#### Scenario: 過期但內文不變只換時戳

- **WHEN** 手冊已存在，一頁 sources 為 `[desktop-app]`、generated 為 2026-09-05，規格 desktop-app 有 2026-09-10T17:15:24+08:00 的 @trace updated，但該頁重生後內文與磁碟相同
- **THEN** 該頁只有 generated 行改為本次時戳，sources 仍為 `[desktop-app]`，摘要 `Stamp-only` 列計入該頁、`Regenerated` 不計入；index.md 與 about.md 隨之重生

#### Scenario: 錨定失效的頁不走只換時戳

- **WHEN** 手冊已存在，一頁 sources 為 `["desktop-app#Old Heading"]` 而正式規格已把該 Requirement 改名
- **THEN** 該頁整頁重寫、sources 改為現行標題並計入 `Regenerated`，不計入 `Stamp-only`

#### Scenario: 錨定頁不受他段封存影響

- **WHEN** 手冊已存在，一頁 sources 為 `["desktop-app#Quality Gates"]`、generated 為 2026-09-05，規格 desktop-app 只有另一段帶 2026-09-10T17:15:24+08:00 的 @trace updated
- **THEN** 技能的過期報告不列該頁，也不重讀該規格的 Requirement 內文

#### Scenario: 無異動時零寫入

- **WHEN** 手冊已存在且無任何過期頁或未入冊能力
- **THEN** 無檔案被寫入，摘要明示手冊已是最新

---
### Requirement: 導覽模式的行為

<!-- BEFORE: 無手冊時明示繁體中文「尚無手冊，改以規格直接導覽」。 -->

導覽模式 SHALL NOT 寫入任何檔案。有手冊時 SHALL 讀取各頁 frontmatter 為索引，先以一個問題確認使用者角色，再依 section 與 order 帶領旅程，每個回答 SHALL 附出處（capability 名或手冊頁名）。無手冊時 SHALL 以英文範例句 `No manual yet — touring directly from the specs` 明示並以 speclink list --specs 與 speclink show 讀規格作答。導覽結束時的後續建議（例如跑生成模式產出手冊）SHALL 明文為僅建議、SHALL NOT 自動呼叫其他技能。此為刻意變更：無手冊提示範本改為英文並同批更新 golden 快照。

#### Scenario: 有手冊的導覽

- **WHEN** speclink/manual/ 存在且使用者呼叫導覽模式
- **THEN** 技能先問角色一題，依手冊順序導覽，回答附出處，過程中零檔案寫入

#### Scenario: 無手冊的導覽

- **WHEN** speclink/manual/ 不存在且使用者呼叫導覽模式
- **THEN** 技能以英文範例句 `No manual yet — touring directly from the specs` 明示並改以規格直接導覽，過程中零檔案寫入

---
### Requirement: remote 模式的限制敘述

<!-- BEFORE: remote 模式不支援手冊生成的說明句以繁體中文「remote 模式尚不支援手冊生成（導覽模式不受此限：/speclink:manual 導覽）」撰寫。 -->

技能 SHALL 於生成模式開始前判斷專案是否綁定 remote store（.speclink.yaml 含 remote 區段）：綁定時 SHALL 以英文範例句 `Manual generation is not yet supported in remote mode (tour mode still works: /speclink:manual tour)` 明示並停止，零檔案寫入；導覽模式不受此限。技能檔 SHALL 敘明此限制。以「規格資料夾不在這個 checkout」渲染的技能檔（remote 專案，或內嵌引擎而沒有本機規格檔的宿主）SHALL 另外在生成模式的開頭直接寫明這份技能檔的專案不支援手冊生成、輸出同一句英文說明後結束且零檔案寫入，SHALL NOT 依賴 .speclink.yaml 的讀取結果；導覽模式同樣不受此限。以「規格資料夾在這個 checkout」渲染的技能檔 SHALL NOT 含這一段，執行時的判斷照舊。此為刻意變更：不支援提示範本改為英文並同批更新 golden 快照，remote 行為與條件區塊邏輯不變。

#### Scenario: remote 綁定專案的生成被擋

- **WHEN** 於 remote 綁定的專案呼叫生成模式
- **THEN** 技能輸出英文不支援說明並停止，speclink/manual/ 未被建立或改動

#### Scenario: remote 綁定專案的導覽照常

- **WHEN** 於 remote 綁定的專案呼叫導覽模式
- **THEN** 導覽照常進行（以規格或既有手冊為索引），零檔案寫入

#### Scenario: 沒有 .speclink.yaml 的內嵌宿主也被擋

- **WHEN** 以「規格資料夾不在這個 checkout」渲染 manual 技能，檢視生成模式的第一個步驟
- **THEN** 步驟在讀取 .speclink.yaml 的判斷之前就寫明英文不支援說明並結束，不寫入任何檔案；導覽模式不受影響

#### Scenario: 本機專案的 manual 技能沒有多出來的拒絕

- **WHEN** 以「規格資料夾在這個 checkout」渲染 manual 技能
- **THEN** 生成模式的第一個步驟只有執行時讀取 .speclink.yaml 的判斷，不含無條件拒絕手冊生成的額外段落
