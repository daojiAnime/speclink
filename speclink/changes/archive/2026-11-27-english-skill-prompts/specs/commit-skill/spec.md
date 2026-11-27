## MODIFIED Requirements

### Requirement: 先封存子流程的順序提示與收尾提醒

<!-- BEFORE: 先封存子流程的順序提示、重寫提醒與下一個可開工提醒引文以繁體中文撰寫。 -->

內嵌 speclink-commit 技能（事實來源 crates/engine/speclink-core/assets/skills/commit.md，經 init 與 update 渲染至工具技能目錄）的「先封存再一起提交」子流程 SHALL 在執行 speclink archive 之前執行 speclink plan --json，並全數以與 archive 技能同源語意的英文範例句撰寫提醒：目標 change 的 `archiveAfter` 非空時 SHALL 提醒先封存 `<archiveAfter names>` 並重讀同名 requirement 區塊對照正式規格以 ingest 重寫後再封存 `<name>`；`requirementOverlap` 含 `conflict` 為 true 者時 SHALL 提醒兩邊都新增（或改名成）、或都移除（或改名掉）同名 requirement，須先修改其中一邊；`blockedBy` 非空時 SHALL 另提宣告前置 `<blockedBy names>` 尚未封存（`Declared prerequisite <blockedBy names> is not yet archived.`）；三者皆為僅建議——SHALL NOT 阻擋、使用者確認後照常封存，SHALL NOT 指示以 drift 作為補救；三者皆空、目標不在 plan 內或 plan 失敗時 SHALL NOT 提及順序。子流程 SHALL 同時記下 `archiveAfter` 含目標 change 的其他 change。子流程封存成功後 SHALL 以英文敘明三件事：工作區存在 speclink/manual/ 時提醒以 manual 技能檢查手冊是否過期（條件僅為目錄存在，僅提醒、SHALL NOT 代跑）；封存前記下的 change 非空時提醒它們先對照正式規格以 ingest 重寫再封存（僅提醒、SHALL NOT 單獨觸發詢問、SHALL NOT 代跑 ingest）；再執行 speclink plan --json，`next` 非 null 時以英文範例 `Next ready to start from plan: <next> — run /speclink:apply <next>.` 提醒，有效 worktree 政策開啟且第 1 波兩個以上可開工時以 `Wave 1 parallel-safe:` 列可並行名單，`next` 為 null 或 plan 失敗時不提；皆僅提醒、SHALL NOT 代跑。此為刻意變更：子流程提醒範本改為英文並同批更新 golden 快照，語意與分支條件不變。

#### Scenario: 子流程封存前的順序提示

- **WHEN** 使用者於 commit 技能選擇「先封存再一起提交」，目標 change 在 plan 中的 archiveAfter 為 ["add-a"]
- **THEN** 技能檔指示在執行 speclink archive 前以英文提醒先封存 add-a 並於 add-a 封存後對照正式規格以 ingest 重寫再封存 `<name>`，使用者確認後照常封存

#### Scenario: 無阻擋不提

- **WHEN** 目標 change 的 archiveAfter、blockedBy 皆為空且無 conflict
- **THEN** 技能檔指示不提及順序，直接進入封存

#### Scenario: 子流程封存後的收尾提醒

- **WHEN** 子流程封存成功、工作區存在 speclink/manual/、封存前 add-c 的 archiveAfter 含目標 change、plan 的 next 為 add-b
- **THEN** 技能檔指示提醒手冊過期檢查、提醒 add-c 先對照正式規格重寫再封存，並提供英文提醒 `Next ready to start from plan: add-b — run /speclink-apply add-b.`，三者皆僅提醒
