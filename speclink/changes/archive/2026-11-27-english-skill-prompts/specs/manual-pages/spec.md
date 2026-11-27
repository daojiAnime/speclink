## MODIFIED Requirements

### Requirement: 內文慣例

<!-- BEFORE: 每頁結尾出處行固定以繁體中文 `**出處**：` 開頭，前端只剝離 `**出處**：`。 -->

頁內文 SHALL 使用 GitHub Flavored Markdown。提示框 SHALL 用 GitHub Alert 語法（`> [!NOTE]`、`> [!TIP]`、`> [!WARNING]`、`> [!CAUTION]`）。跨頁連結 SHALL 用相對檔名（例：`[Layout Overview](layout.md)`）。生成端於每頁結尾 SHALL 寫出一行以 `**Sources**:` 開頭的來源行，逐一以反引號列出與 frontmatter `sources` 相同的 capability 名（例：`**Sources**: \`github-oauth\`, \`user-pending-blocked-pages\``）。讀取端剝離來源行時 SHALL 同時相容英文 `**Sources**:`、簡體中文 `**出处**：` 與既有繁體中文 `**出處**：`（半形 `:` 與全形 `：` 皆須支援），使既有舊手冊頁不破版、無須回改。內文 SHALL NOT 含 HTML 標籤；SHALL NOT 出現 `--json` 欄位名、旗標或程式碼細節，除非該內容是使用者實際輸入的指令或技能名。此為刻意變更：生成端來源行契約字面改為英文 `**Sources**:`，讀取端向後相容既有中文來源行。

#### Scenario: 出處行與 sources 一致

- **WHEN** 一頁的 `sources` 為 `[github-oauth, user-pending-blocked-pages]`
- **THEN** 該頁最後一個非空段落為 `**Sources**:` 開頭且恰含這兩個反引號名稱

#### Scenario: 提示框與連結形式

- **WHEN** 檢視任一頁內文
- **THEN** 提示框皆為 GitHub Alert 語法、跨頁連結皆為相對 `.md` 檔名，且全文無 HTML 標籤

#### Scenario: 讀取端向後相容新舊來源行

- **WHEN** 讀取端處理最後一個非空段落分別為 `**Sources**: \`a\``、`**出处**：\`a\``、`**出處**：\`a\`` 或不含來源行的四份手冊內文
- **THEN** 前三份的尾端來源行皆被完整剝離，第四份原樣回傳不改動任何字元

---
### Requirement: 必產的首頁與來源頁

<!-- BEFORE: about.md 的 title 為「本手冊的來源」，無矛盾標記為「未發現」，無使用者面向能力提示為「尚無可入冊的使用者面向能力」。 -->

每份手冊 SHALL 含 `index.md`（`title` 為手冊名、`section` 為第一個分區、`order` 為全手冊最小值；內容為系統一句話定位、三個以內的核心概念、依角色分流的入口連結）與 `about.md`（`title` 為 `About this manual`、`order` 為全手冊最大值；內容 SHALL 載明取材範圍為 `speclink/specs/`、規格內部新舊描述矛盾的清單（無矛盾時明寫 `None found`）、已知侷限（無截圖、以實機為準）與編成日期）。兩頁為衍生頁：任一其他頁新增或重生時 SHALL 隨之重生（`section` 與 `order` 保留），且 SHALL NOT 被判為過期、未入冊或來源消失。此為刻意變更：來源頁契約字面量改為英文，既有工作區的 `about.md` 仍正常由 slug 識別與渲染。

#### Scenario: 生成後兩頁存在

- **WHEN** 生成端完成任一次生成
- **THEN** `speclink/manual/index.md` 與 `speclink/manual/about.md` 存在，`index.md` 的 `order` 為全手冊最小、`about.md` 的 `title` 為 `About this manual` 且 `order` 為最大，且 `about.md` 含矛盾清單段（無矛盾時含 `None found`）

#### Scenario: 無可入冊能力時仍產兩頁

- **WHEN** 規格中無任何被分流為使用者面向的 capability
- **THEN** 仍產出 `index.md` 與 `about.md`，`about.md` 載明 `No user-facing capabilities yet`，無其他頁
