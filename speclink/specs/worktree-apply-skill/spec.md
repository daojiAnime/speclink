# worktree-apply-skill Specification

## Purpose

/speclink-apply-with-worktree 技能的內容：技能檔如何由 apply 本體加上 worktree 前後段組合生成、前置指示（政策檢查、變更存在性、產物入 HEAD、worktree 路徑與分支慣例），以及收尾指示（在 worktree 內提交、不自行合併、交棒給 worktree-merge）。本 capability 保證平行作業的每個變更各自待在自己的 worktree 裡，主 checkout 全程不被動到。

## Requirements

### Requirement: apply-with-worktree 技能的生成與組合

技能再生（speclink 的技能同步機制）SHALL 產出 apply-with-worktree 技能：claude 目標生成 .claude/skills/speclink-apply-with-worktree/SKILL.md，codex 目標生成對應產物。生成內容 SHALL 為自包含文件：worktree 前置段、完整的 apply 本體流程（與 apply 技能同源的全文，非摘要亦非引用）、worktree 收尾段，三段依序組合。既有 apply 技能的生成輸出 SHALL 維持位元級不變。

#### Scenario: claude 目標生成技能檔

- **WHEN** 於已初始化 claude 工具的 workspace 執行技能再生
- **THEN** .claude/skills/speclink-apply-with-worktree/SKILL.md 存在，內容含 worktree 前置段與 apply 本體流程的既有段落標記

#### Scenario: apply 本體同源不走樣

- **WHEN** 技能再生後比對 speclink-apply-with-worktree 與 speclink-apply 的生成內容
- **THEN** 前者完整包含後者的本體流程段落（逐段可對應），且後者的輸出與本能力導入前位元級一致


<!-- @trace
source: worktree-parallel-apply
updated: 2026-08-04
-->

---
### Requirement: apply-with-worktree 技能的前置指示

生成的技能內文 SHALL 全數以英文指示執行代理依序完成前置：(0) 解析輸入——偵測到多於一個 change 名時 SHALL 先執行 `speclink plan --json`，把 `blockedBy` 為空的名字列成英文並行配方（`The parallel workflow is one change per session: open separate windows and run /speclink:apply-with-worktree <change-name> in each.`）、`blockedBy` 非空的名字逐一以英文列出其前置並說明須等前置落地再開（`<change-name> must wait until <prerequisites> lands before starting.`），然後停下請使用者自並行名單擇一；恰有一個可並行時 SHALL 改為請使用者確認是否在此執行該 change（不印配方），沒有可並行者時 SHALL 列出須等與不可用的名字並停止；SHALL NOT 靜默依序批次執行多個 change；(1) 讀取有效 worktree 政策（含 SPECLINK_WORKTREE 環境覆寫層），值非 true 時 SHALL 拒絕執行並以英文說明 `This project has not enabled the worktree workflow. To enable it, run: speclink workflow-config set worktree true`，SHALL NOT 逕行在主資料夾執行 apply 本體；(2) 以 plan 選 change 並守門，同時揭露並發重疊——於主 checkout 執行 `speclink plan --json`：未指名時 SHALL 取 `next`，`next` 為 null 時 SHALL 列出每個 change 的 `blockedBy` 並停止；指名時 SHALL 讀該 change 的 `blockedBy`，非空時 SHALL 印出前置清單並停止；指名的 change 不在 `changes` 內（已封存、不存在、或列於 `skipped`）時 SHALL 停止並列出可用的 change 名；plan 失敗（依賴成環、專案未初始化）時 SHALL 顯示錯誤並停止；通過守門後 SHALL 一併檢查 `plan --json` 的 `overlaps`、`requirementOverlap`、`archiveAfter` 及 `speclink worktree status --json`（`wave1Overlaps` 與活躍 worktree 清單，必要時輔以 `git worktree list --porcelain`），若存在並發重疊的活躍 sibling worktree 或衝突/重疊提醒（含 `sharedPaths`、`sharedSubmodules`、`conflict: true`），SHALL 於開啟 worktree 前清楚向使用者揭露重疊範圍並建議將品質關卡（`/speclink:review`、`/speclink:verify`、`/speclink:quality`）延後至 `/speclink:worktree-merge` 回到主 checkout 後再蓋章；本步驟 SHALL 位於產物提交與建立 worktree 之前，任一停止情況 SHALL NOT 提交產物、SHALL NOT 建立 worktree；(3) 確認該 change 的產物目錄已提交進 HEAD——未提交時僅提交該 change 目錄本身，SHALL NOT 夾帶其他髒檔（worktree 由 HEAD 具現化，產物不在 HEAD 的 worktree 不含該 change）；(3.5) 讀取該 change 的 evidence 檔案清單並對主樹查 git 狀態——無檔案清單或清單為空時靜默續行；任一已觸碰檔於主樹為髒時 SHALL 停下列出髒檔，以推薦序提供三個英文選項：`Commit code first, then open worktree`（recommended）、`Continue anyway`、`Stop`，SHALL NOT 未經使用者選擇逕行建立 worktree；(4) 以分支 `speclink/<change名>` 於 sibling 巢 `<repo資料夾名>.worktrees/<change名>/` 建立 worktree（該分支或 worktree 已存在時沿用既有者續作，SHALL NOT 重複建立），若倉庫含 active submodules 或 `.gitmodules`，SHALL 於 worktree 內執行 `git -C "<worktree-path>" submodule update --init --recursive` 完成初始化／同步；(5) 向使用者印出英文建置成本提示（worktree 為完整原始碼副本，須自備依賴安裝與建置產物）；(6) 其後的 apply 本體流程以英文分隔句 `The apply workflow follows below; execute it inside the worktree directory above.` 引出並於該 worktree 資料夾內執行，且內文 SHALL 明寫：本體第 1 步在 worktree 內執行的 plan 查詢只是複查——worktree 內的 plan 讀不到其他 worktree 的進度與主線後續的封存——其結果與 (2) 不同時 SHALL 以主 checkout 的判定為準，本體第 2 步（含 `review prepare` 與 `in-progress add`）SHALL 照常執行，且若實作會修改 submodule 內檔案，SHALL 於該 submodule 內先切出／建立 `speclink/<change名>` 分支再改動（避免留在 detached `HEAD`）。apply 本體資產 SHALL NOT 因本要求而改動，speclink-apply 的生成輸出除 frontmatter 的 version 行外 SHALL 維持位元級不變。此為刻意變更：前置段所有繁體中文提示改為英文並同批更新 golden 快照，語意與步驟順序不變。

#### Scenario: 內文含政策拒跑指示

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 內文含英文政策拒跑指示 `This project has not enabled the worktree workflow` 與 `workflow-config set worktree true` 字面

#### Scenario: 內文含建立慣例與續作指示

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 內文含分支慣例 speclink/<change名>、sibling 巢路徑慣例，與「已存在即沿用續作」的指示

#### Scenario: 內文含產物先進 HEAD 指示

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 內文含「change 產物提交進 HEAD」的指示，且該指示位於建立 worktree 步驟之前

#### Scenario: 內文含多 change 拒收指示

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 內文含以 plan 分出可並行與須等前置者、英文多 session 配方 `one change per session` 與等待句 `must wait until`、恰有一個可並行時改為確認、無可並行者停止與禁止靜默依序批次的指示，且位於政策檢查步驟之前

#### Scenario: 內文含進度與程式碼分家偵測指示

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 內文含讀取該變更已觸碰檔案清單並對主樹查 git 狀態的指示，與三個英文選項推薦序（`Commit code first, then open worktree`、`Continue anyway`、`Stop`），且位於產物進 HEAD 步驟之後、建立 worktree 步驟之前

#### Scenario: 內文含 plan 選擇與守門指示

- **WHEN** 技能再生後讀取 speclink-apply-with-worktree 的 SKILL.md
- **THEN** 前置段含執行 `speclink plan --json`、未指名取 `next`、`blockedBy` 非空即停止、不在 `changes` 即停止並列可用名單的指示，且該指示位於政策檢查步驟之後、產物進 HEAD 步驟之前，並含不提交產物與不建立 worktree 的指示

#### Scenario: 內文含並發重疊揭露與 submodule 準備指示

- **WHEN** 技能再生後讀取 speclink-apply-with-worktree 的 SKILL.md
- **THEN** 前置段含檢查 `speclink worktree status --json` 與 `overlaps`/`requirementOverlap`/`archiveAfter` 以揭露並發重疊並建議將品質蓋章延後至主 checkout 合併後的指示，含 `git -C "<worktree-path>" submodule update --init --recursive` 初始化指示，以及提醒將 submodule 修改置於 `speclink/<change名>` 分支的指示

#### Scenario: 內文含本體複查以主 checkout 為準指示

- **WHEN** 技能再生後讀取 speclink-apply-with-worktree 的 SKILL.md
- **THEN** 前置段於英文分隔句 `The apply workflow follows below; execute it inside the worktree directory above.` 之前含本體第 1 步在 worktree 內只是複查、結果不同時以主 checkout 的判定為準、第 2 步照常執行的指示，且 speclink-apply 的 SKILL.md 除 frontmatter 的 version 行外與本變更前位元級一致

##### Example: 多 change 名依 plan 分組

| 輸入的 change 名 | plan 的 blockedBy | 代理人的處置 |
| --- | --- | --- |
| add-a、add-b、add-c | add-a：空；add-b：空；add-c：[add-a] | 並行配方列 add-a、add-b；add-c 說明 `must wait until add-a lands before starting`；請使用者自 add-a、add-b 擇一 |
| add-a、add-c | add-a：空；add-c：[add-a] | add-c 說明 `must wait until add-a lands before starting`；不印配方，請使用者確認是否在此執行 add-a |
| add-b、add-c | add-b：[add-a]；add-c：[add-a] | add-b、add-c 各說明 `must wait until add-a lands before starting`；沒有可選的 change，停止；不提交產物、不建立 worktree |
| add-c | add-c：[add-a] | 印出前置 add-a 並停止；不提交產物、不建立 worktree |
| （未指名） | next 為 add-a | 選 add-a，宣告 `Using change: add-a` 後續行 (3) |
| （未指名） | next 為 null | 列出每個 change 的 blockedBy 並停止 |


<!-- @trace
source: english-skill-prompts
updated: 2026-11-27T19:46:25+08:00
-->

---
### Requirement: apply-with-worktree 技能的收尾指示

生成的技能內文 SHALL 指示執行代理於 apply 本體完成後：於 worktree 內完成該 change 的提交（沿用 commit 技能的歸屬慣例），SHALL NOT 執行合併回主分支，SHALL NOT 移除 worktree。若 worktree 內有任何 submodule 處於髒狀態，SHALL 先於該 submodule 內檢查並確認改動屬於此 change，確保 submodule 位於 `speclink/<change名>` 分支（若為 detached `HEAD` 則建立或沿用該分支），先於 submodule 內提交，再於 superproject 暫存更新後的 `160000` gitlink 指標與其他屬於此 change 的檔案並提交。

提交之後、交棒之前，技能內文 SHALL 指示檢查並發重疊並視情況把分支 rebase 到主分支最新位置、重設品質關卡起點：

1. 先檢查審查與驗證工單（`speclink review show`、`speclink verify show`）與並發 worktree／重疊狀態（`speclink worktree status --json`、`speclink plan --json`、`git worktree list --porcelain`）。任一站已有工單時 SHALL 略過整步 rebase——不在品質迴圈中途 rebase；若有並發重疊的活躍 sibling worktree，應在交棒中提醒使用者。
2. 合併目標取主 checkout 目前所在的分支；該分支為 speclink/* 或 detached 時 SHALL 略過並說明。
3. 記下合併目標當下的 commit，於 worktree 內 rebase 到該 commit 所在的分支；發生 `160000` submodule gitlink 衝突或真實檔案衝突時，SHALL 執行 `git rebase --abort`、回報衝突檔案／submodule 並略過重設起點（分支未變，原起點仍有效；`160000` 歧異交由後續 `/speclink:worktree-merge` 的 submodule-first merge 處理），SHALL NOT 代解衝突。
4. rebase 成功後 SHALL 同步其餘乾淨且未被此分支改動的 submodule（`git submodule update --init --recursive` 或對乾淨 submodule 同步），並於 worktree 內執行 `speclink review prepare <change> --base <記下的 commit>`。

交棒 SHALL 以明確文字依變更與並發狀態建議路徑，由使用者判斷是否執行品質關卡，且交棒句 SHALL 明列三個入口：review（工藝品質）∥ verify（規格符合度），或 quality（兩站合跑）：

- 若偵測到並發重疊的活躍 sibling worktree：優先建議先執行 `/speclink:worktree-merge` 回到主 checkout 後，再執行品質關卡（避免在其他並發分支落地隨即失效的 fork-point hash 上重複蓋章）。
- 無並發重疊 worktree 且沒有未勾的 `[M]` 手動任務：於 worktree 內執行品質關卡，蓋章寫入的 meta 變更 SHALL 提示補提交，再以 worktree-merge 技能收尾。
- 尚有未勾的 `[M]` 手動任務（`speclink instructions apply` 的 `remaining` 減 `codeRemaining` 大於零），或使用者想在蓋章前看到 CI 結果（worktree 分支只在本機、從不 push，CI 看不到它）：先以 worktree-merge 技能收尾，再於主 checkout 執行品質關卡、完成手動任務後封存——合併時基準會承接到主 checkout。

#### Scenario: 內文含停點與正典順序交棒指示

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 內文含「不合併、不移除 worktree」的停點指示，交棒段的品質關卡建議明列 review、verify、quality 三個入口，依並發重疊與有無未勾手動任務分流（含並發重疊時建議延後至合併後於主 checkout 蓋章、「worktree 內執行、蓋章後補提交、再 worktree-merge」與「先 worktree-merge、再於主 checkout 執行」），並點名 worktree-merge 技能

#### Scenario: 內文含提交後 rebase、160000 處理與重設起點指示

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 內文含「先於 `speclink/<change>` 提交 submodule 再暫存 `160000` gitlink」「兩站皆無工單才 rebase」「check `speclink worktree status --json` 偵測並發重疊」「rebase 衝突（含 `160000`）即 abort、略過重設起點、不代解」與「成功後同步乾淨 submodule 並執行 `speclink review prepare <change> --base <記下的 commit>`」的指示

#### Scenario: 交棒提及 CI 看不到 worktree 分支

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 交棒段載明想在蓋章前看到 CI 結果時走「先合併」路徑，理由為 worktree 分支只在本機、從不 push


<!-- @trace
source: quality-gate-speedup
updated: 2026-10-14T21:31:03+08:00
-->

---
### Requirement: worktree 技能的規格檔步驟依 checkout 產生

apply-with-worktree 技能的前置指示中，「變更的 artifact 尚未 commit 時先 commit」的步驟與其 git 指令 SHALL 只在規格資料夾位於這個 checkout 時產生；不位於時 SHALL 以一句說明取代：規格文件在 Server 上，沒有東西要先 commit。前置指示檢查「這個變更已經改到的檔案在主 checkout 是否乾淨」時，檔案清單 SHALL 取自 speclink evidence show <change> --json 的 files。收尾指示要 stage 的檔案 SHALL 是 evidence 的檔案；規格資料夾位於這個 checkout 時 SHALL 另外加上這個變更在規格資料夾底下的檔案。對應的守則句子 SHALL 在兩種情況各自與步驟一致。

#### Scenario: 本機專案保留先 commit 的步驟

- **WHEN** 以「規格資料夾在這個 checkout」產生 apply-with-worktree 技能
- **THEN** 前置指示含先 commit 變更的 artifact 的步驟與對規格資料夾的 git 指令

#### Scenario: remote 專案沒有先 commit 的步驟

- **WHEN** 以「規格資料夾不在這個 checkout」產生 apply-with-worktree 技能
- **THEN** 前置指示不含對規格資料夾的 git 指令，並含一句說明規格文件在 Server 上

#### Scenario: 已改到的檔案以動詞取得

- **WHEN** 檢視兩種情況產生的前置指示中讀取變更記錄的步驟
- **THEN** 步驟以 speclink evidence show 表述，不含 cat 檔案的指令

#### Scenario: 收尾的 stage 範圍

- **WHEN** 檢視兩種情況產生的收尾指示
- **THEN** 「不在」的版本只 stage evidence 的檔案；「在」的版本另外 stage 這個變更在規格資料夾底下的檔案

<!-- @trace
source: skills-verbs-second-wave
updated: 2026-11-12T16:54:51+08:00
-->