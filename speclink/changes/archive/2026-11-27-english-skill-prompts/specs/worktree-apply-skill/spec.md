## MODIFIED Requirements

### Requirement: apply-with-worktree 技能的前置指示

<!-- BEFORE: 政策拒絕訊息、多變更等待／並行配方、髒工作樹三選項、建置成本提示與本體分隔句以繁體中文撰寫。 -->

生成的技能內文 SHALL 全數以英文指示執行代理依序完成前置：(0) 解析輸入——偵測到多於一個 change 名時 SHALL 先執行 `speclink plan --json`，把 `blockedBy` 為空的名字列成英文並行配方（`The parallel workflow is one change per session: open separate windows and run /speclink:apply-with-worktree <change-name> in each.`）、`blockedBy` 非空的名字逐一以英文列出其前置並說明須等前置落地再開（`<change-name> must wait until <prerequisites> lands before starting.`），然後停下請使用者自並行名單擇一；恰有一個可並行時 SHALL 改為請使用者確認是否在此執行該 change（不印配方），沒有可並行者時 SHALL 列出須等與不可用的名字並停止；SHALL NOT 靜默依序批次執行多個 change；(1) 讀取有效 worktree 政策（含 SPECLINK_WORKTREE 環境覆寫層），值非 true 時 SHALL 拒絕執行並以英文說明 `This project has not enabled the worktree workflow. To enable it, run: speclink workflow-config set worktree true`，SHALL NOT 逕行在主資料夾執行 apply 本體；(2) 以 plan 選 change 並守門——於主 checkout 執行 `speclink plan --json`：未指名時 SHALL 取 `next`，`next` 為 null 時 SHALL 列出每個 change 的 `blockedBy` 並停止；指名時 SHALL 讀該 change 的 `blockedBy`，非空時 SHALL 印出前置清單並停止；指名的 change 不在 `changes` 內（已封存、不存在、或列於 `skipped`）時 SHALL 停止並列出可用的 change 名；plan 失敗（依賴成環、專案未初始化）時 SHALL 顯示錯誤並停止；本步驟 SHALL 位於產物提交與建立 worktree 之前，任一停止情況 SHALL NOT 提交產物、SHALL NOT 建立 worktree；(3) 確認該 change 的產物目錄已提交進 HEAD——未提交時僅提交該 change 目錄本身，SHALL NOT 夾帶其他髒檔（worktree 由 HEAD 具現化，產物不在 HEAD 的 worktree 不含該 change）；(3.5) 讀取該 change 的 evidence 檔案清單並對主樹查 git 狀態——無檔案清單或清單為空時靜默續行；任一已觸碰檔於主樹為髒時 SHALL 停下列出髒檔，以推薦序提供三個英文選項：`Commit code first, then open worktree`（recommended）、`Continue anyway`、`Stop`，SHALL NOT 未經使用者選擇逕行建立 worktree；(4) 以分支 speclink/<change名> 於 sibling 巢 <repo資料夾名>.worktrees/<change名>/ 建立 worktree（該分支或 worktree 已存在時沿用既有者續作，SHALL NOT 重複建立）；(5) 向使用者印出英文建置成本提示（worktree 為完整原始碼副本，須自備依賴安裝與建置產物）；(6) 其後的 apply 本體流程以英文分隔句 `The apply workflow follows below; execute it inside the worktree directory above.` 引出並於該 worktree 資料夾內執行，且內文 SHALL 明寫：本體第 1 步在 worktree 內執行的 plan 查詢只是複查——worktree 內的 plan 讀不到其他 worktree 的進度與主線後續的封存——其結果與 (2) 不同時 SHALL 以主 checkout 的判定為準，且本體第 2 步（含 review prepare 與 in-progress add）SHALL 照常執行。apply 本體資產 SHALL NOT 因本要求而改動，speclink-apply 的生成輸出除 frontmatter 的 version 行外 SHALL 維持位元級不變。此為刻意變更：前置段所有繁體中文提示改為英文並同批更新 golden 快照，語意與步驟順序不變。

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
