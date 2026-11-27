## MODIFIED Requirements

### Requirement: 設定頁政策下拉的未知值顯性呈現

<!-- BEFORE: 合法選項集僅含 tw／ja／en（spec_locale 另含 auto），無 cn 選項。 -->

專案設定產出流程頁的 locale 與 spec_locale 下拉（local 與遠端專案皆適用）在儲存值非空且不在合法選項集（locale：`cn`／`tw`／`ja`／`en`；spec_locale：`cn`／`tw`／`ja`／`en`／`auto`）時，SHALL 於下拉顯示該原始值並帶無效標註與警示樣式，且該欄位下方 SHALL 顯示引導改選合法代碼（含 `cn`）的提示文字；SHALL NOT 呈現為空白，SHALL NOT 於讀取時自動清空或改寫儲存值（寫入嚴格、讀取寬容）。使用者改選合法選項並儲存後，SHALL 以所選代碼覆蓋原值，下拉 SHALL 恢復正常呈現且提示文字 SHALL 消失。儲存值為空（未設定）或在合法選項集（含 `cn`，顯示標籤 `cn（简体中文）`）內時，本需求 SHALL NOT 顯示無效標註或提示文字。此為刻意變更：合法選項集與無效提示同步納入 `cn`。

#### Scenario: 未知儲存值顯性呈現且不被改寫

- **WHEN** 專案的 locale 儲存值為「繁體中文」（合法選項集外的字串），使用者開啟專案設定的產出流程頁
- **THEN** locale 下拉顯示「繁體中文」並帶無效標註與警示樣式，欄位下方出現改選合法代碼（含 `cn`）的提示文字；未執行任何儲存動作前，儲存端的值仍為「繁體中文」

#### Scenario: 改選合法代碼即修復

- **WHEN** 於上述狀態，使用者將 locale 下拉改選 `cn`（或 `tw`）並儲存
- **THEN** 儲存端的 locale 值成為所選代碼，下拉正常顯示對應選項，無效標註與提示文字消失

#### Scenario: 合法值與未設定不受影響

- **WHEN** 專案的 locale 儲存值為 `cn`、spec_locale 為 `auto`（或未設定），使用者開啟專案設定的產出流程頁
- **THEN** locale 下拉正常顯示 `cn（简体中文）`、spec_locale 顯示對應合法選項或未設定預設狀態，無任何無效標註或提示文字
