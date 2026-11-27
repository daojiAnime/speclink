## MODIFIED Requirements

### Requirement: 封存完成後的收尾提交提醒

<!-- BEFORE: 封存後提示「plan 的下一個可開工：<next>，執行 /speclink-apply <next>」與第 1 波可並行繁體中文話術。 -->

內嵌 speclink-archive 技能（事實來源 crates/engine/speclink-core/assets/skills/archive.md，經 init 與 update 渲染至工具技能目錄）SHALL 於結尾敘明：封存完成後提醒使用者以一般提交收尾本次封存產生的異動（delta 併入正式規格、變更目錄搬移至 archive）——commit 技能的變更選檔流程不適用於封存後，SHALL NOT 導向之。此提醒 SHALL 涵蓋所有進入封存的路徑（apply 直達、review、verify、quality、worktree-merge 之後），且 SHALL 明文為僅提醒——SHALL NOT 代跑提交。技能檔 SHALL 另敘明：工作區存在 speclink/manual/ 時，提醒使用者以 manual 技能檢查手冊是否因本次封存而過期——條件僅為該目錄存在（不判斷本次封存動到哪些規格），且 SHALL 明文為僅提醒、SHALL NOT 代跑 manual 技能。技能檔 SHALL 再敘明「下一個可開工」提示（以英文模板撰寫，由對話語言段落指示模型依對話語言呈現）：封存完成後執行 speclink plan --json，`next` 非 null 時提供英文範例句 `Next ready to start from plan: <next> — run /speclink:apply <next>.`；有效 worktree 政策（含 SPECLINK_WORKTREE 環境覆寫層）開啟且第 1 波（waves[0].changes）中 blockedBy 為空且未開工的 change 有兩個以上時，SHALL 以英文範例句 `Wave 1 parallel-safe: <name-a>, <name-b> — open one session per change and run /speclink:apply-with-worktree <name>.` 列出可並行名單；`next` 為 null 或 plan 失敗（依賴成環）時 SHALL NOT 提及順序。此提示 SHALL 明文為僅提醒、SHALL NOT 代跑 apply。此為刻意變更：技能檔輸出改為英文範本並同批更新 golden 快照，語意與條件分支維持不變。

#### Scenario: 技能檔結尾含提交提醒

- **WHEN** 檢視渲染產出的 speclink-archive 技能檔結尾段
- **THEN** 內文含封存完成後提醒提交的指示，且明文僅提醒、不代跑提交

#### Scenario: 手冊存在時的過期檢查提醒

- **WHEN** 檢視渲染產出的 speclink-archive 技能檔結尾段對手冊的敘述
- **THEN** 內文含「工作區存在 manual 目錄時提醒以 manual 技能檢查手冊是否過期」的英文指示，條件為目錄存在，且明文僅提醒、不代跑

#### Scenario: 封存後提示下一個可開工

- **WHEN** 檢視渲染產出的 speclink-archive 技能檔結尾段對 plan 的敘述
- **THEN** 內文含執行 speclink plan --json、next 非 null 時以英文範例 `Next ready to start from plan: <next>` 提示、worktree 政策開啟且第 1 波兩個以上可開工時以 `Wave 1 parallel-safe:` 列可並行名單、next 為 null 或 plan 失敗時不提的指示，且明文僅提醒、不代跑

##### Example: 提示的分岔

| plan 結果 | worktree 政策 | 提示（英文範本） |
| --- | --- | --- |
| next = add-a，第 1 波可開工只有 add-a | 開或關 | `Next ready to start from plan: add-a — run /speclink-apply add-a.` |
| next = add-a，第 1 波可開工為 add-a、add-b | 開 | 上句加 `Wave 1 parallel-safe: add-a, add-b — open one session per change and run /speclink-apply-with-worktree <name>.` |
| next = add-a，第 1 波可開工為 add-a、add-b | 關 | 只提 add-a |
| next = null | 開或關 | 不提順序 |
| plan 回成環錯誤 | 開或關 | 不提順序 |

---
### Requirement: 未指名時的候選清單依 plan 順序

<!-- BEFORE: 候選狀態標籤與封存順序提示引文以繁體中文「可封存」「等 <名稱> 封存」「前置 <名稱> 未封存」「metadata 壞掉」撰寫。 -->

內嵌 speclink-archive 技能 SHALL 於未指名 change 時以 speclink plan --json 的 `changes` 陣列列出候選（依配置順序），每個候選 SHALL 以英文標籤標出其 `archiveAfter`（空時標 `Ready to archive`，非空時標 `Waiting on archive of <archiveAfter names>`），`blockedBy` 非空時 SHALL 再標 `Prerequisite <blockedBy names> not yet archived`，`skipped` 項目附於清單尾端並標 `Broken metadata`；SHALL 仍由使用者選擇、SHALL NOT 自動選。封存前的順序提示（`archiveAfter` 建議先封存並重寫重疊 requirement、`conflict` 提醒先改其中一邊、`blockedBy` 提醒尚未封存）與封存後提醒排在後面的變更重寫，SHALL 皆以英文範例句撰寫且維持既有語意與非阻擋性質。plan 失敗（依賴成環）時 SHALL 退回 speclink list --json 列候選。其他技能（commit、review、verify、drift、analyze）的候選清單 SHALL 維持 list --json，不受本需求影響。此為刻意變更：標籤與範例句改為英文並同批更新 golden 快照。

#### Scenario: 候選依 plan 順序並標阻擋

- **WHEN** 未指名執行 speclink-archive 技能，plan 的 changes 依序為 add-a（archiveAfter 空）、add-b（archiveAfter ["add-a"]）
- **THEN** 候選清單依 add-a、add-b 順序呈現，add-a 標 `Ready to archive`、add-b 標 `Waiting on archive of add-a`，並由使用者選擇

#### Scenario: plan 失敗退回 list

- **WHEN** 未指名執行 speclink-archive 技能且 plan 回依賴成環錯誤
- **THEN** 候選清單改以 speclink list --json 的順序呈現，不標阻擋
