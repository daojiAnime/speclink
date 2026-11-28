# worktree-merge-skill Specification

## Purpose

/speclink-worktree-merge 技能的內容：技能檔的生成，以及收尾流程的指示——確認主樹乾淨、把 worktree 分支合併回主分支，成功後移除 worktree 並刪掉分支。本 capability 保證合併是使用者顯式觸發的一步，衝突時停下來交回給人判斷而不自行裁決。

## Requirements

### Requirement: worktree-merge 技能的生成

技能再生 SHALL 產出 worktree-merge 技能：claude 目標生成 .claude/skills/speclink-worktree-merge/SKILL.md，codex 目標生成對應產物。此技能 SHALL 為獨立完整模板（不組合其他技能本體）。

#### Scenario: claude 目標生成技能檔

- **WHEN** 於已初始化 claude 工具的 workspace 執行技能再生
- **THEN** .claude/skills/speclink-worktree-merge/SKILL.md 存在


<!-- @trace
source: worktree-parallel-apply
updated: 2026-08-04
-->

---
### Requirement: worktree-merge 技能的收尾流程指示

生成的技能內文 SHALL 指示執行代理依序收尾指定 change 的 worktree：(1) preflight——執行 `speclink worktree preflight "<change-name>" --target "<target-branch>" --json` 並搭配檢查主資料夾當前分支非 `speclink/*` 且非 detached（合併目標分支 SHALL 於合併前向使用者宣告，SHALL NOT 代為切換分支）、`git status --porcelain`、`git submodule status` 與該 change 的 worktree（含其內 active submodule）是否皆已提交；若主資料夾唯一未提交的變更是先前記錄的已落地 `.meta.yaml`（`main.cleanExceptLandedMeta` / `main.dirtyLandedMeta`），SHALL 採建設性處理而非死鎖（若該變更 `readyToArchive` 為 `true` 且符合封存條件則先以 `/speclink:archive` 封存並提交，否則將該 `.meta.yaml` 單獨暫存並以正常 commit 提交，不使用 `--no-verify`）；若主資料夾有其他無關的髒檔、主資料夾或 worktree 內有髒 submodule、worktree 分支有未提交變更、或需要 submodule 合併時 submodule `HEAD` 為 detached，SHALL 立即停止並說明原因，SHALL NOT 代為 stash 或代為提交無關變更；(2) 分流處理 `160000` submodule gitlink 歧異與 rebase-first 階梯——合併開始前 SHALL 先記下主資料夾的 HEAD commit（合併前 HEAD），並檢查 preflight 回傳的 `submoduleConflicts` 與 `fileConflicts`：若 `fileConflicts` 非空（真實原始碼衝突），SHALL 直接回報並停止（SHALL NOT 試圖自動解衝突）；若無 `fileConflicts` 但 `submoduleConflicts` 非空（雙方僅 `160000` submodule gitlink 指標分歧），SHALL 略過單純在 superproject 執行且必撞 `160000` 的 rebase（或於 rebase 僅因 `160000` 撞車且 `fileConflicts` 為空時執行 `git rebase --abort`），改於主資料夾走 **submodule-first merge**：先以 `git merge --no-ff --no-commit`（或無須 ff 時 `git merge --no-commit`）啟動 superproject 合併、針對每個分歧的 submodule 若 `<wt-sub-sha>` 尚未存在於主庫則先從 worktree submodule fetch，再於 `<主 checkout>/<submodule>` 內執行 `git merge <wt-sub-sha>`（若 submodule 內發生真實內容衝突 SHALL 立即中止 submodule 與 superproject 兩邊的合併並回報），否則於 superproject 暫存解開後的 `160000` gitlink 指標並以 `git commit --no-edit` 完成合併提交；若無 `submoduleConflicts`，則維持於 worktree 內將分支 `speclink/<change名>` rebase 到合併目標分支，成功後於主資料夾以 `git merge --ff-only` 落地（線圖不產生合併節點），rebase 衝突時 SHALL 執行 `git rebase --abort` 退回主資料夾一般 merge，fast-forward 被拒時亦同並告知使用者本次留下合併節點；任一 fast-forward 或一般合併成功後，對 `worktree.submodulesChanged` 中其餘乾淨且尚未同步的 submodule SHALL 同步至合併後的目標 commit（`git -C "<main>/<sub-path>" merge --ff-only <target-sha>` 或 `git submodule update --init "<sub-path>"`）；(3) 一般 merge 或 submodule merge 發生真實原始碼衝突時 SHALL 立即停止並中止未完成的合併、回報衝突檔案清單，SHALL NOT 代編衝突內容；「不代解 rebase 衝突」「不代解 merge 衝突」與「不得僅因可解的 `160000` gitlink 歧異而直接中止放棄」SHALL 同列於守則；(4) 合併成功後、移除 worktree 之前，SHALL 於主資料夾依序執行 `speclink change landed`（帶 `--branch` 與合併後 HEAD `--commit`）與 `speclink review prepare <change> --base <合併前 HEAD> --from <worktree 路徑>`（該指令失敗時 SHALL 回報並停止，SHALL NOT 移除 worktree）；完成後移除該 worktree 並刪除 superproject 的 `speclink/<change名>` 分支，同時檢查並刪除其提交已可從目標分支觸及的 submodule 內同名 `speclink/<change名>` 分支；(5) 完成主資料夾收尾並確認輸出（標示本次以 fast-forward、合併節點或 submodule-first 合併落地、主 checkout 已恢復乾淨），依變更狀態交棒：已依規則完成封存者報明已封存；品質關卡已完成且未過期者→下一步為主 checkout `/speclink:archive`；品質關卡尚未執行或因並發分支落地而延後／重設者→於主 checkout 執行品質關卡，基準已承接，SHALL NOT 稱之為降級路徑；尚有未勾 `[M]` 手動任務→於主 checkout 執行品質關卡、完成手動任務後再封存。

技能內文的前提敘述 SHALL NOT 宣稱所有步驟都不在 worktree 內執行——步驟 (2) 的 rebase 即以 worktree 為對象。

#### Scenario: 內文含 preflight 停止與建設性 `.meta.yaml` 處理指示

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 內文含執行 `speclink worktree preflight "<change-name>" --target "<target-branch>" --json` 的指示；明示無關髒檔、髒 submodule、worktree 未提交或 detached submodule HEAD 時立即停止且不代為 stash／提交無關變更；同時明示當主資料夾僅剩先前記錄的已落地 `.meta.yaml` 未提交（`main.cleanExceptLandedMeta` / `main.dirtyLandedMeta`）時不因此死鎖，可先封存（若 `readyToArchive`）或單獨正常提交該 `.meta.yaml`

#### Scenario: 內文含合併目標分支確認指示

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 內文含「確認主資料夾當前分支」的指示（branch --show-current），且明示停在 speclink/* 或 detached、不代為切換分支

#### Scenario: 內文含 rebase-first 與 160000 submodule-first 合併階梯指示

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 內文含依 `submoduleConflicts` 與 `fileConflicts` 分流：`fileConflicts` 非空即停；純 `160000` gitlink 歧異改走主資料夾 submodule-first merge（視需要 fetch `<wt-sub-sha>`、於 `<main>/<sub-path>` 合併、submodule 內容衝突則雙邊中止、成功後暫存 `160000` 指標並完成合併）；無 submodule 衝突時維持「worktree 內先 rebase 合併目標分支」與「成功後主資料夾以 `--ff-only` 合併」的階梯，且於 rebase／merge 後同步 `worktree.submodulesChanged` 裡其餘乾淨 submodule

#### Scenario: 內文含 fast-forward 被拒的出口

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 內文含「fast-forward 被拒時退回一般 merge、並告知使用者本次留下合併節點」的指示，且前提敘述未宣稱所有步驟都不在 worktree 內執行

#### Scenario: 守則清單含 rebase 紅線、submodule 紅線與落地方式標示

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 守則清單含「不代解 rebase 衝突」「不代解 merge／submodule 內容衝突」與「不得僅因可解的 `160000` gitlink 歧異而直接中止」，且成功輸出區塊標示本次以 fast-forward、合併節點或 submodule-first 合併落地

#### Scenario: 內文含衝突即停指示

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 內文含「一般 merge 或 submodule merge 發生真實原始碼衝突時立即停止、回報衝突檔案、不代編、中止兩邊未完成合併後回報」的指示

#### Scenario: 內文含清理與正典順序交棒指示

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 內文含「合併成功後移除 worktree 並刪除 superproject 與對應 submodule 的 `speclink/<change>` 分支」的指示；交棒段依狀態分岔——已依規則完成封存者報明已封存、品質關卡已完成且未過期者以主 checkout `/speclink:archive` 為下一步、尚未執行或因並發落地重設基準／延後者於主 checkout 執行品質關卡（基準已承接）、有未勾手動任務者先跑品質關卡與手動任務再封存；內文不含「降級路徑」的說法

#### Scenario: 內文含合併後寫入基準指示

- **WHEN** 技能再生後讀取 SKILL.md
- **THEN** 內文含「合併開始前記下主資料夾 HEAD」與「合併成功後、移除 worktree 前執行 `speclink review prepare <change> --base <合併前 HEAD> --from <worktree 路徑>`」的指示，並明示該指令失敗時停止、不移除 worktree


<!-- @trace
source: quality-gate-speedup
updated: 2026-11-28T20:28:00+08:00
-->

---
### Requirement: 合併後回報落地

worktree-merge 技能 SHALL 在變更分支合併進主分支成功之後，指示執行 `speclink change landed <change-name> --branch speclink/<change-name>` 回報落地（`--commit` 為合併後主分支的 HEAD）。技能 SHALL 指示先以 git 取得合併後主分支的 HEAD，並以 `--commit` 明確帶上：技能的動詞可能在沒有 checkout 的宿主那一端執行，那裡讀不到 HEAD。合併失敗或中止時 SHALL NOT 回報。回報被拒絕時，技能 SHALL 指示把訊息告訴使用者並繼續收尾，SHALL NOT 還原合併。規格資料夾在 checkout 裡時，回報與 `review prepare` 會寫進主 checkout 的 `{{SPEC_DIR}}changes/<change-name>/.meta.yaml`；為避免把主 checkout 留成髒樹而阻塞後續並發 worktree，技能 SHALL 指示於 worktree 移除後完成主 checkout 收尾——若該變更已就緒（品質關卡新鮮或略過、無未完成 `[M]` 手動任務、無未結品質工單、且 `plan --json` 無 `archiveAfter`／衝突阻擋）則依規則以 `/speclink:archive <change-name>` 封存並提交，否則將該 `.meta.yaml` 暫存並以正常 commit（不使用 `--no-verify`）提交，使主 checkout 恢復乾淨。

#### Scenario: 合併成功後回報

- **WHEN** 檢視產生的 worktree-merge 技能中合併成功之後的步驟
- **THEN** 有一步執行 `speclink change landed`，帶 `--branch speclink/<change-name>`，並以 `--commit` 帶上合併後主分支的 HEAD

#### Scenario: 合併失敗不回報

- **WHEN** 檢視合併發生衝突而中止的分支
- **THEN** 該分支沒有回報落地的步驟

#### Scenario: 回報被拒絕時繼續收尾

- **WHEN** 檢視回報那一步的失敗處理
- **THEN** 它指示告訴使用者並繼續，沒有指示還原合併

#### Scenario: 本機專案為主 checkout 提交或封存落地資料檔

- **WHEN** 分別檢視為「規格資料夾在 checkout 裡」與「不在 checkout 裡」產生的 worktree-merge 技能
- **THEN** 前者在收尾步驟指示對主 checkout 內的 `{{SPEC_DIR}}changes/<change-name>/.meta.yaml` 依就緒狀態執行 `/speclink:archive` 或單獨正常提交，使主 checkout 保持乾淨以免阻塞後續並發 worktree；後者沒有這段對本機 `.meta.yaml` 檔提交或封存的說明

<!-- @trace
source: station-status-and-landing
updated: 2026-11-28T20:28:00+08:00
-->