## MODIFIED Requirements

### Requirement: checkout 綁定驗證與 marker 寫入

<!-- BEFORE: checkout 步驟勾選清單為 Claude／Codex／Copilot／Pi／Cursor，尚未涵蓋 omp。 -->

選擇連接 checkout 時 SHALL 先以零寫入檢查資料夾狀態：含 remote marker（`.speclink.yaml` 的 remote section）時，其 URL origin 與 repo SHALL 與所選 scope 一致；不一致 SHALL 以繁中訊息拒絕並指出 marker 指向。無 marker 時資料夾 SHALL 為 Git repository 方可繼續，非 Git repository SHALL 拒絕。檢查成功後，checkout 步驟 SHALL 顯示 Claude／Codex／Copilot／Pi／Cursor／omp checkbox 及資料夾路徑；既有 `.speclink.yaml` 的 built-in tools SHALL 成為選取值，缺少 tools 清單時 SHALL 只依實際工具 footprint 預選且 SHALL NOT 默認 Claude；`.github/` 的存在 SHALL NOT 使 Copilot 被預選，`.pi/`、`.cursor/`、`.omp/` 的存在 SHALL NOT 使 Pi、Cursor、omp 被預選。至少一個 built-in 工具與 checkout 路徑齊備前，「開啟 Workspace」SHALL disabled。

提交後 Desktop SHALL 重做 marker 邊界驗證，將 built-in 選集同步至 `.speclink.yaml`，生成或更新所選工具的 Skills、剝除 `AGENTS.md`／`CLAUDE.md` 的遺留 Speclink 區塊（不生成任何指令檔），並清理未選工具的 Speclink 受管產物；custom descriptor、unknown entry、remote／spec_dir／其他設定及指令區塊外的使用者內容 SHALL 保留。無 marker checkout SHALL 寫入與 CLI Remote init 同構的 remote section，既有相符 marker checkout SHALL 仍執行同步，不得提前成功。全部同步成功後 remote locator 的 checkoutRoot SHALL 記錄該資料夾並隨分頁持久化；同步失敗時 SHALL 保持 chooser、路徑與選集供重試，SHALL NOT 建立 remote tab／session 或開始 handshake。分頁 SHALL 以最小面（tooltip）呈現已連接的 checkout 路徑。checkout 綁定 SHALL NOT 改變本階段 capability 可用性。

#### Scenario: marker 不一致拒絕

- **WHEN** 所選資料夾的 remote marker 指向不同 origin 或不同 repo
- **THEN** 唯讀檢查被拒且訊息指出 marker 指向的 origin 與 repo，磁碟內容不變，分頁不建立 checkout 關聯

#### Scenario: 無 marker 的 Git repository 先選工具再綁定

- **WHEN** 使用者選取無 marker 的 Git repository，檢查完成後勾選 Claude 與 Codex並按下「開啟 Workspace」
- **THEN** 按下前資料夾零寫入；提交成功後 `.speclink.yaml` 含兩個 built-in tools 與所選 scope 的 remote section，兩組 Skills 存在且不生成 `AGENTS.md`、`CLAUDE.md`，CLI 可進入相同 Remote 模式，且 Workspace 才開始 handshake

#### Scenario: checkout 不允許空工具選集

- **WHEN** checkout 路徑已檢查成功但 Claude、Codex、Copilot、Pi、Cursor 與 omp 均未選取
- **THEN** 「開啟 Workspace」維持 disabled，資料夾內容不變且沒有 tab／session 建立

#### Scenario: 既有相符 marker 缺少 Skills 時補齊

- **WHEN** checkout 的 remote marker 與 scope 相符、built-in tools 為 `[codex]`，但 Codex Skills 缺少，使用者保持 Codex 選取並提交
- **THEN** 缺少的 Codex 受管產物被補齊，remote section 值不變，不建立 `speclink/`，同步成功後才開啟 Workspace

#### Scenario: 既有 checkout 從 Claude 切換為 Codex

- **WHEN** checkout 原 built-in tools 為 `[claude]`，`CLAUDE.md` 同時含 Speclink 區塊與使用者文字，使用者改為只選 Codex並提交
- **THEN** `.speclink.yaml` built-in tools 成為 `[codex]`，Codex Skills 被補齊且不生成 `AGENTS.md`，Claude Skills 與 `CLAUDE.md` 的遺留 Speclink 區塊被移除，`CLAUDE.md` 使用者文字與 remote section 保留，且不存在本機 `speclink/`

#### Scenario: 同步失敗不開啟且可重試

- **WHEN** checkout 檢查成功，但同步受管產物時遇到檔案系統寫入錯誤
- **THEN** chooser 顯示含失敗階段的單行錯誤並保留路徑與工具選集，remote tab／session／handshake 均未建立；修正檔案系統後以相同選集再次提交可收斂並開啟

#### Scenario: 勾選 Copilot 綁定 checkout

- **WHEN** 使用者選取無 marker 的 Git repository（含 `.github/workflows/ci.yml`），檢查完成後 Copilot 未被預選，使用者勾選 Claude 與 Copilot 並按下「開啟 Workspace」
- **THEN** `.speclink.yaml` 的 built-in tools 為 claude 與 copilot，`.claude/skills/` 與 `.github/skills/` 兩組 Skills 存在，`.github/workflows/ci.yml` 位元級不變，不生成 `.github/copilot-instructions.md`、`AGENTS.md`、`CLAUDE.md`

#### Scenario: 勾選 Cursor 綁定 checkout

- **WHEN** 使用者選取無 marker 的 Git repository（含 `.cursor/rules/style.mdc`），檢查完成後 Cursor 未被預選，使用者勾選 Claude 與 Cursor 並按下「開啟 Workspace」
- **THEN** `.speclink.yaml` 的 built-in tools 為 claude 與 cursor，`.claude/skills/` 與 `.cursor/skills/` 兩組 Skills 存在，`.cursor/rules/style.mdc` 位元級不變，不生成 `.cursor/agents/`、`AGENTS.md`、`CLAUDE.md`

#### Scenario: 勾選 omp 綁定 checkout

- **WHEN** 使用者選取無 marker 的 Git repository（含 `.omp/rules/team.md`），檢查完成後 omp 未被預選，使用者勾選 Claude 與 omp 並按下「開啟 Workspace」
- **THEN** `.speclink.yaml` 的 built-in tools 為 claude 與 omp，`.claude/skills/` 與 `.omp/skills/` 兩組 Skills 存在，`.omp/rules/team.md` 位元級不變，不生成 `.omp/agents/`、`AGENTS.md`、`CLAUDE.md`

---
### Requirement: remote marker 資料夾的探測分流

<!-- BEFORE: 探測分流的內建工具選集列舉為 Claude／Codex／Copilot／Pi／Cursor，未包含 omp。 -->

專案探測 SHALL 辨識 remote marker。資料夾僅含 marker 且對應 connection 已登入時，若 `.speclink.yaml` 有至少一個有效 built-in tool，Desktop SHALL 先依該選集 reconciliation，成功後才 handshake 開啟 remote tab並以呼叫端原始 path 作為 checkoutRoot；同步失敗 SHALL 顯示錯誤且不建立 session。marker 缺少有效 built-in tool 選集時 SHALL 導向 chooser 的 checkout 步驟、預填 server／scope／path 並要求明示選擇 Claude／Codex／Copilot／Pi／Cursor／omp，不得直接開啟。無對應 connection 或未登入 SHALL 引導至 chooser 的 server 步驟並預填 server 位址。

marker 與本地 `speclink/` 並存時 SHALL 停下強制選擇，提供三個出口且皆無靜默覆蓋：「繼續本地」（以本地開啟、marker 不動）；「以 server 為準」（本地 `speclink/` 改名為帶日期備份後，資料夾轉為 checkout，完成工具 reconciliation 後開啟 remote tab，不上傳本地內容、不改動 server）；「遷移本地內容」（進入 workspace-migration 的遷移流程、目標為空 scope）。對話文案 SHALL 明說「以 server 為準」為備份後棄用本地、非合併。marker YAML 損壞 SHALL 沿 `.speclink.yaml` 既有 fail-closed 語意呈現錯誤。

「繼續本地」開出的本地分頁 SHALL 延續這個選擇：之後切回這個分頁、以快捷鍵或系統匣切換到它，以及 app 重開時（這個分頁是上次作用中的分頁，或在背景接受啟動時的路徑有效性探測），資料夾仍是本地 `speclink/` 與 marker 並存者 SHALL 照樣以本地開啟並照常計算背景分頁的待收尾數，SHALL NOT 轉錯誤態、SHALL NOT 再次停下強制選擇、SHALL NOT 發出 handshake。這個延續只適用於分頁路徑本身就是並存專案根的分頁：分頁路徑是並存專案根底下的子目錄，或探索命中的專案根不是分頁路徑時，本地分頁 SHALL 轉錯誤態，SHALL NOT 開出上層專案的內容。本地規格資料夾已不存在、只剩 marker 時，本地分頁 SHALL 照舊轉錯誤態。經「新增專案」或最近開啟清單再次開啟同一個並存資料夾時，SHALL 照舊停下強制選擇。

#### Scenario: 有工具選集的 checkout 直達 remote tab

- **WHEN** 開啟僅含 remote marker 的資料夾、built-in tools 為 `[codex]` 且對應 server 已登入
- **THEN** Desktop 先補齊或更新 Codex 受管產物，成功後不經 chooser 完成 handshake，remote tab 開啟且 checkoutRoot 為該資料夾

#### Scenario: 缺少工具選集時導向 checkout 選擇

- **WHEN** 開啟僅含 remote marker 的資料夾、tools 缺席或不含任何內建工具（claude、codex、copilot、pi、cursor、omp），且對應 server 已登入
- **THEN** Desktop 導向 chooser checkout 步驟並預填原 path 與 scope，內建工具選集需由使用者確認，且提交成功前不建立 remote tab／session

#### Scenario: 自動補齊失敗不 handshake

- **WHEN** 有 built-in tools 的既有 checkout 在 reconciliation 時遇到檔案系統錯誤
- **THEN** Desktop 顯示帶路徑與失敗階段的錯誤，remote tab／session 不建立且不發出 handshake

#### Scenario: 並存衝突三出口

- **WHEN** 開啟同時含本地 `speclink/` 與 remote marker 的資料夾
- **THEN** 呈現強制選擇對話含三出口：繼續本地以本地開啟；以 server 為準將本地改名備份後執行工具 reconciliation 再開 remote tab且 server 內容未變；遷移本地內容進入遷移流程；無任何自動覆蓋

#### Scenario: 繼續本地的分頁切回仍以本地開啟

- **WHEN** 對同時含本地 `speclink/` 與 remote marker 的資料夾選「繼續本地」，切到另一個專案後再點回這個分頁的方塊
- **THEN** 這個分頁成為作用中並顯示本地看板，方塊沒有錯誤態，沒有出現強制選擇對話，也沒有發出 handshake

#### Scenario: 重開 app 後繼續本地的分頁不轉錯誤態

- **WHEN** 圖示列有一個「繼續本地」開出的並存資料夾分頁，關掉 app 再開；分別試這個分頁是上次作用中的分頁，與它在背景兩種情況
- **THEN** 兩種情況下這個分頁都沒有錯誤態：上次作用中時以本地看板開啟；在背景時方塊照常呈現，點下去以本地開啟

#### Scenario: 本地規格資料夾移除後切回轉錯誤態

- **WHEN** 「繼續本地」開出的分頁所在資料夾，本地 `speclink/` 被刪除、只剩 remote marker，之後切回這個分頁
- **THEN** 方塊轉為錯誤態，提示「已不是 speclink 專案」，不發出 handshake

#### Scenario: 並存專案子目錄的分頁切回轉錯誤態

- **WHEN** 「繼續本地」開出的分頁路徑是並存專案根底下的子目錄（或是巢狀專案：它的本地 `speclink/` 已刪除，而上層資料夾是並存專案），之後切回這個分頁
- **THEN** 方塊轉為錯誤態，提示「已不是 speclink 專案」，不開出上層專案的內容，不發出 handshake

#### Scenario: 再次開啟並存資料夾仍強制選擇

- **WHEN** 圖示列已有「繼續本地」開出的並存資料夾分頁，使用者再經「新增專案」選取同一個資料夾
- **THEN** Desktop 照舊停下強制選擇並呈現三出口，選擇前不切換作用中專案

#### Scenario: 壞 marker fail-closed

- **WHEN** 選取的 checkout 具有無法解析的 `.speclink.yaml`
- **THEN** Desktop 顯示解析錯誤，不修改資料夾、不建立 tab／session且不發出 handshake

#### Scenario: 只選 Copilot 的 checkout 直達 remote tab

- **WHEN** 開啟僅含 remote marker 的資料夾、built-in tools 為 `[copilot]` 且對應 server 已登入
- **THEN** Desktop 先補齊或更新 `.github/skills/` 下的 Copilot 受管產物，成功後不經 chooser 完成 handshake，remote tab 開啟且 checkoutRoot 為該資料夾

#### Scenario: 只選 Pi 的 checkout 直達 remote tab

- **WHEN** 開啟僅含 remote marker 的資料夾、built-in tools 為 `[pi]` 且對應 server 已登入
- **THEN** Desktop 先補齊或更新 `.pi/skills/` 下的 Pi 受管產物，成功後不經 chooser 完成 handshake，remote tab 開啟且 checkoutRoot 為該資料夾

#### Scenario: 只選 omp 的 checkout 直達 remote tab

- **WHEN** 開啟僅含 remote marker 的資料夾、built-in tools 為 `[omp]` 且對應 server 已登入
- **THEN** Desktop 先補齊或更新 `.omp/skills/` 下的 oh-my-pi 受管產物，成功後不經 chooser 完成 handshake，remote tab 開啟且 checkoutRoot 為該資料夾
