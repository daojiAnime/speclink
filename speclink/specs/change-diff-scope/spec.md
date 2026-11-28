# change-diff-scope Specification

## Purpose

品質關卡審查面的界定語意：Apply 開始前記錄的 host-local baseline、以 git 解析完整 worktree patch 的 discovery scope、把 discovery 與 validation 綁在一起的凍結快照，以及 review scope 的人眼與 --json 兩路契約。本 capability 保證審查面可重現且範圍歧義時 fail closed——只能以 hash-pinned selection 解鎖，remote workspace 與本機共用同一 resolver。

## Requirements

### Requirement: Apply 開始前記錄 host-local baseline

Apply 技能 SHALL 在執行 in-progress add 前呼叫 speclink review prepare <change>（不帶旗標）。該子指令 SHALL 不讀 stdin；不帶旗標時的行為如本條所述，`--base`／`--from` 兩個旗標的行為見「以可信 commit 重設 Apply baseline 並承接快照」。不帶旗標時，prepare 於 `.speclink/review-scopes/<change>/baseline.json` 記錄 version、change、baseCommit、dirtyFilesAtStart、capturedAt、confidence；JSON 欄位 SHALL 為 camelCase。touched v1／v2 記錄、change metadata 與遠端儲存的文件 SHALL NOT 因 baseline 而增欄。

change 尚未帶 started_* 時，prepare SHALL 以當下 Git HEAD、開始前 dirty paths 與 UTC RFC3339 時間原子取代舊 baseline，confidence 為 initial。change 已開始且 baseline 存在時 SHALL 保留首次 baseline；已開始但 baseline 缺失時 SHALL 記 confidence=late；無 Git checkout 時 SHALL 記 confidence=unavailable 且 baseCommit=null。initial 成功 SHALL exit 0 且 stdout 為空；late／unavailable SHALL exit 0、stdout 為空並在 stderr 顯示後續審查需明示 fixed point。寫入失敗或 change 不存在 SHALL 非零結束，且 Apply 技能 SHALL NOT 接著執行 in-progress add。

#### Scenario: 首次 Apply 記錄乾淨 baseline

- **WHEN** 任務尚未開始、HEAD 為 40 字元 commit SHA，worktree 有 `notes/local.txt` 一個既存髒檔，Apply 技能開始執行
- **THEN** prepare exit 0、stdout 為空，baseline 的 baseCommit 為該 SHA、dirtyFilesAtStart 為 ["notes/local.txt"]、confidence 為 initial，touched 記錄不存在

#### Scenario: 已開始但 baseline 缺失

- **WHEN** change 已帶 started_at 且 host-local baseline 不存在時執行 review prepare
- **THEN** command exit 0、stderr 說明 baseline 為 late，記錄不得被 resolver 當成可信自動歸屬

#### Scenario: baseline 寫入失敗停止 Apply 起點

- **WHEN** `.speclink/review-scopes/<change>` 無法寫入
- **THEN** review prepare 非零結束，in-progress add 不執行，change metadata 維持原狀


<!-- @trace
source: retire-teamstore-and-drivers
updated: 2026-11-17T22:53:25+08:00
-->

---
### Requirement: Git-backed discovery scope 解析完整 worktree patch

speclink review scope <change> SHALL 以 touchedFiles 联集作自动候选路径，使用可信 baseCommit 到目前 worktree 的 Git diff，涵盖 staged、unstaged、rename、delete、touched untracked additions 与 git submodule（tree mode 160000 gitlink）；SHALL NOT 使用只比较 commit graph 的 `<base>...HEAD`。规格资料夹内的 artifacts 与 `.speclink` work data SHALL NOT 成为 review target。

成功的文字 file delta SHALL 回报 oldPath、newPath、kind、beforeHash、afterHash 与 hunks；每个 hunk SHALL 含 id、oldStart、oldLines、newStart、newLines。addition SHALL 允許 oldStart=0、oldLines=0；deletion SHALL 允許 newStart=0、newLines=0；多段修改 SHALL 保留多笔 hunk。rename SHALL 保留 oldPath 與 newPath。binary SHALL 回报 file hashes 與 kind=binary，hunks 为空。

tree mode 160000 的 gitlink 路径 SHALL 作为不透明指针解析，回报 `kind="submodule"`、`hunks=[]` 与对应的 `oldPath`/`newPath`、`beforeHash`/`afterHash`（新增 gitlink 的 `beforeHash` 为 null，删除 gitlink 的 `afterHash` 为 null），SHALL NOT 对 gitlink 调用父仓对象内容读取，也 SHALL NOT 将子模块目录当作普通文件读取。gitlink 的 `beforeHash` SHALL 由 baseCommit 树中该路径记录的 160000 commit SHA 的规范字节计算；`afterHash` SHALL 由当前工作区 gitlink 规范状态字节计算：已初始化且干净的子模块为当前 HEAD commit SHA，已初始化且工作区有未提交改动的子模块为当前 HEAD commit SHA 加工作区确定性脏态摘要，未初始化子模块为父仓索引记录的 160000 commit SHA。不含 gitlink 的仓库在人眼输出、`--no-color` 输出与 `--json` 输出上 SHALL 与修改前逐位元保持一致。

#### Scenario: 未提交 staged 與 unstaged 內容都在 scope

- **WHEN** touched path 同时有 staged 与 unstaged 修改，baseCommit 等于 Apply 开始时 HEAD
- **THEN** review scope 的 patch 同时含两部分修改，即使 `git diff <base>...HEAD` 对该 fixture 为空

#### Scenario: untracked touched file 是整檔 addition

- **WHEN** touchedFiles 含一个未追踪 UTF-8 档案
- **THEN** resolved payload 将它列为 added，beforeHash 为 null，首个 hunk 的 oldStart=0、oldLines=0

#### Scenario: delete 與 rename 保留雙端語意

- **WHEN** touched scope 含一个删除档与一个 Git 可辨识 rename
- **THEN** delete 的 afterHash 为 null 且 newLines=0，rename 同时输出 oldPath 与 newPath

#### Scenario: 子模块工作区脏改动解析为 submodule delta

- **WHEN** 父仓 baseCommit 中 `frontend` 为 160000 gitlink，Apply 期间仅在 `frontend` 子模块工作区产生未提交改动（子模块 HEAD 未变）且 `frontend` 位于 touchedFiles，执行 `speclink review scope <change> --json`（不读 stdin）
- **THEN** exit code 为 0，stdout 为合法 JSON 且 `state="resolved"`、`patchHash` 以 `sha256:` 开头、`paths` 含 `"frontend"`，`files` 中该项的 `oldPath="frontend"`、`newPath="frontend"`、`kind="submodule"`、`hunks=[]`、`beforeHash` 与 `afterHash` 均非 null 且互不相等；`.speclink/review-scopes/<change>/snapshots/` 新增对应快照文件

#### Scenario: 子模块指针推进与删除保留双端哈希

- **WHEN** touchedFiles 同时包含一个已提交并推进指针的子模块 `sub-a` 与一个已从父仓删除的子模块 `sub-b`，执行 `speclink review scope <change> --json`
- **THEN** exit code 为 0，`sub-a` 的 `kind="submodule"`、`hunks=[]`、`beforeHash` 与 `afterHash` 非 null 且互不相等；`sub-b` 的 `kind="submodule"`、`oldPath="sub-b"`、`newPath=null`、`beforeHash` 非 null、`afterHash=null`、`hunks=[]`

#### Scenario: 未初始化子模块以父仓索引指针计算哈希

- **WHEN** 子模块目录 `vendor-sub` 尚未初始化（目录内无 `.git`），但父仓索引将其指针更新为新的 160000 commit，执行 `speclink review scope <change> --json`
- **THEN** exit code 为 0，`vendor-sub` 解析为 `kind="submodule"`、`hunks=[]`、`beforeHash` 与 `afterHash` 均非 null 且互不相等，不因缺少子模块 `.git` 或目录不可读而报错

##### Example: gitlink 规范状态哈希分流

| 子模块状态 | `beforeHash` 来源 | `afterHash` 来源 |
| --- | --- | --- |
| 干净且指针推进（A → B） | `Subproject commit A\n` 的 SHA-256 | `Subproject commit B\n` 的 SHA-256 |
| HEAD 未变但工作区脏（A → A-dirty） | `Subproject commit A\n` 的 SHA-256 | `Subproject commit A-dirty\ndirty-worktree: sha256:<digest>\n` 的 SHA-256 |
| 未初始化且索引更新（A → B） | `Subproject commit A\n` 的 SHA-256 | `Subproject commit B\n` 的 SHA-256 |
| 新增子模块（无 → B） | `null` | 当前工作区规范字节的 SHA-256 |
| 删除子模块（A → 无） | `Subproject commit A\n` 的 SHA-256 | `null` |

---


<!-- @trace
source: support-gitlink-in-scope
updated: 2026-11-28T13:50:37+08:00
-->

---
### Requirement: review scope 的 human 與 JSON 契約

speclink review scope <change> SHALL 支援 `--json`、`--base <rev>`、`--candidate-hash <sha256>` 與可重複的 `--include-hunk <id>`，且 SHALL NOT 讀取 stdin。自動歸屬成功時 command SHALL exit 0 並建立 frozen snapshot。

`--json` resolved payload SHALL 含 change:string、phase:"discovery"|"validation"、state:"resolved"、baseCommit:string、candidateHash:string、patchHash:string、paths:string[]、files:array、patch:string、outOfScopeChanged:string[]（恆存在，無範圍外變動時為空陣列）；files 每項 SHALL 含 oldPath:string|null、newPath:string|null、kind:string、beforeHash:string|null、afterHash:string|null、hunks:array；validation phase 的 files 每項 SHALL 另含 attribution:"finding"|"adjacent"|"new"，discovery phase SHALL 缺席該欄位；hunks 每項 SHALL 含 id:string 與四個 number ranges。human 成功路徑 SHALL 將 phase、patchHash、路徑數與 hunk 數寫至 stdout；outOfScopeChanged 非空時 SHALL 另以一行列出範圍外變動路徑。兩條路徑於 `--no-color` 下 SHALL 不含 ANSI。

#### Scenario: JSON resolved payload 可供 reviewer 使用

- **WHEN** 對可自動歸屬的兩檔三 hunk change 執行 review scope --json
- **THEN** exit 0、stdout 為合法 JSON、state 為 resolved、paths 有兩項、hunks 合計三項且 patchHash 以 `sha256:` 開頭

#### Scenario: validation payload 帶出身標記與範圍外註記

- **WHEN** 驗證輪的修復動到一個 findings 檔與一個未點名候選檔，另有一個 discovery 時被排除的檔也變動
- **THEN** resolved payload 的 files 分別帶 attribution "finding" 與 "adjacent"，outOfScopeChanged 含被排除檔的路徑且該檔不在 files 中

#### Scenario: 找不到 change

- **WHEN** 對不存在的 change 執行 review scope
- **THEN** command 非零結束、stderr 說明找不到 change，stdout 為空且不得建立 baseline 或 snapshot


<!-- @trace
source: review-validation-scope-movement
updated: 2026-08-05
-->

---
### Requirement: 歧義 scope 必須 fail closed 並以 hash-pinned selection 解鎖

baseline 缺失／late／unavailable、base commit 不可解析、touched path 已在 dirtyFilesAtStart、另一 active change 的 touched record 重叠时，review scope SHALL 视为 needsInput，不得建立 frozen snapshot。needsInput SHALL 仅发生于 discovery phase——validation phase 的范围归因依「frozen snapshot 綁定 discovery 與 validation patch」以内容移动解析，SHALL NOT 产生 needsInput。

human 路径 SHALL 非零结束并于 stderr 列出 ambiguous paths，以及可信 `--base`、hash-pinned `--include-hunk`、隔离 worktree 三种处置；当候选 `files` 包含 `kind="submodule"` 条目时，stderr SHALL 另附一行明确说明 submodule 路径无可选文本 hunk、不能通过 `--include-hunk` 钉选，`--no-color` 下不含 ANSI；不含 `submodule` 条目时 stderr 维持原有逐字输出。`--json` 路径 SHALL 将 `state:"needsInput"`、`candidateHash:string|null`、`ambiguousPaths:string[]`、`files:array` 写至 stdout 后非零结束；candidate 可计算时（含候选包含 gitlink 160000 路径的情形）`candidateHash` SHALL 非 null 且 `files` SHALL 列出文本文件的可选 hunk IDs 以及 `kind="submodule"`、`hunks=[]` 的子模块条目。

人工 selection SHALL 同时提供前次 candidateHash 与至少一个 include-hunk。resolver SHALL 重算完整 candidate；hash 不符、hunk ID 不存在、重复 ID、选取 binary 或 submodule delta、或空选择 SHALL 非零拒绝且零 snapshot effects（stderr 说明 binary 与 submodule 均无可选 hunk）。成功 selection SHALL 只把选定文字 hunks 纳入 frozen patch，仍以实际文件 before／after hashes 作漂移锚。

touchedFiles 缺失或为空时 SHALL NOT 自动审查全 worktree；使用者明示 `--base` 后，resolver SHALL 只把整个 diff 当 needsInput candidate，仍须 hash-pinned hunk selection 才能冻结。

#### Scenario: 開始前已髒的 touched file 不被靜默認領

- **WHEN** baseline 的 dirtyFilesAtStart 与 touchedFiles 都含 `src/lib.rs`
- **THEN** review scope 非零结束、state 为 needsInput、ambiguousPaths 含 `src/lib.rs`，snapshots 目录不新增档案

#### Scenario: candidate 漂移拒絕舊選擇

- **WHEN** 第一次 needsInput 回报 candidateHash A，使用者选 hunk 前 worktree 又改变并以 A 重试
- **THEN** command 非零结束、stderr 说明 candidate 已漂移，不建立 snapshot

#### Scenario: hash-pinned hunk selection 成功

- **WHEN** candidateHash 未变且使用者选取同一文字档的两个有效 hunk IDs
- **THEN** command exit 0，resolved patch 仅含该两个 hunks，files 的 beforeHash／afterHash 仍锚定实际整档内容

#### Scenario: 歧义候选含子模块时保留 candidateHash 与文本 hunk

- **WHEN** baseline 的 dirtyFilesAtStart 含 `src/lib.rs`，touchedFiles 含 `src/lib.rs` 与脏子模块 `frontend`，分别以 `--json` 与人类模式（含 `--no-color`）执行 `speclink review scope <change>`
- **THEN** 两次调用均非零结束且不新增 snapshot；`--json` stdout 的 `state="needsInput"`、`candidateHash` 非 null、`files` 同时包含 `src/lib.rs`（含可选 hunk ID）与 `frontend`（`kind="submodule"`、`hunks=[]`）；人类模式 stderr 列出三种处置并另行说明 submodule 不可按 `--include-hunk` 钉选，且 `--no-color` 下无 ANSI 转义码

#### Scenario: 尝试以 include-hunk 选取子模块 delta 被拒绝

- **WHEN** 候选仅含 `frontend`（`kind="submodule"`、`hunks=[]`），使用者传入有效 `--candidate-hash` 与任意 `--include-hunk` 值执行 `speclink review scope <change>`
- **THEN** command 非零结束，stderr 说明未知 hunk ID 且 binary 与 submodule 均无可选 hunk，snapshots 目录不新增档案

---


<!-- @trace
source: support-gitlink-in-scope
updated: 2026-11-28T13:50:37+08:00
-->

---
### Requirement: frozen snapshot 綁定 discovery 與 validation patch

resolved scope SHALL 在 `.speclink/review-scopes/<change>/snapshots/` 原子建立 version 1 snapshot；档名 SHALL 使用 patchHash 的十六进制 digest，不含 `sha256:` 的冒号。snapshot SHALL 记录 change、phase、candidateHash、patchHash、baseCommit、createdAt、dirtyFilesAtCapture path/hash、canonical patch、file deltas、hunk ranges，以及 UTF-8 scope 的 beforeText／afterText；binary 与 submodule SHALL 只记 hashes（不写入 `texts`）。dirtyFilesAtCapture 对普通文件记录原始字节的 `sha256:` 哈希，对 gitlink 160000 路径记录规范 gitlink 状态字节的 `sha256:` 哈希，使账本哈希与对应 delta 的 `afterHash` 严格一致。

工单不存在时 phase SHALL 为 discovery。工单存在时 phase SHALL 为 validation，resolver SHALL 比对上一轮快照 dirtyFilesAtCapture 全部条目的现况杂凑（普通文件读原始字节，gitlink 读规范状态字节），内容移动者 SHALL 进验证 patch：在保存 scope（patch deltas 与 carried texts）内的路径比较 frozen afterText 与目前内容（binary 与 submodule 比较上一轮冻结的 `afterHash` 与当前状态哈希，产生对应的非文本验证段且 `hunks=[]`）；不在保存 scope 但更早某轮快照收录过的路径 SHALL 沿工单各轮 patchHash 链（新→旧）回走，以最近收录该路径的快照重建冻结后状态并输出其差异（attribution `"adjacent"`）；任何轮皆未收录的路径 SHALL NOT 进 patch，SHALL 列入 outOfScopeChanged 注解且不阻挡冻结。再加入上轮 capture 后才变脏的新路径（attribution `"new"`），且已冻结删除的普通文件或 gitlink 仍不存在时 SHALL NOT 重复进入验证补丁。内容未移动的路径 SHALL NOT 进验证 patch。snapshot 缺失（含回走链中任一轮）或工单 patchHash 与 snapshot 不符 SHALL 非零结束，不得退回整档 discovery。

review stamp 或 review discard 成功后 SHALL 清除该 change 的 review snapshots 但保留 Apply baseline；清除失败 SHALL 以 stderr warning 回报，且 SHALL NOT 回滚已完成的工单／metadata mutation。scope 已写 snapshot 但 add-round 失败形成的 orphan SHALL 在下一次无对应工单的 scope 前清除。

#### Scenario: follow-up 只輸出 remediation patch

- **WHEN** Round 1 snapshot 保存 A 与 B，修正只改 A 并新增一个先前干净的 C
- **THEN** validation patch 只含 A 自 frozen afterText 起的差异与 C 的新差异，不重新输出未修改的 B

#### Scenario: 未點名候選檔的修復以 adjacent 段進驗證 patch

- **WHEN** Round 1 candidate 收录 A 与 B，findings 只点名 A，修复同时改动 A 与 B
- **THEN** Round 2 validation patch 含 A（attribution `"finding"`）与 B 自其最近收录轮冻结后状态起的差异（attribution `"adjacent"`），冻结 resolved 而非 needsInput

#### Scenario: 連續多輪修復未點名檔沿雜湊鏈回走

- **WHEN** B 于 Round 2 以 adjacent 段收录后，Round 2 的修复再次改动 B
- **THEN** Round 3 冻结沿 patchHash 链取 Round 2 快照重建 B 的冻结后状态，正常 resolved 输出新差异

#### Scenario: 範圍外變動註記不擋凍結

- **WHEN** discovery 时经 hash-pinned selection 排除的档案于验证期间变动
- **THEN** 验证轮冻结照常 resolved，该档列于 outOfScopeChanged 且不在 patch 中，human 输出含一行范围外变动路径

#### Scenario: snapshot 缺失不退回 discovery

- **WHEN** ticket 的 lastRound.patchHash 存在但 host-local snapshot 已被移除
- **THEN** review scope 非零结束并说明无法精确验收，不能用 touched 整档或目前 worktree 重新 discovery

#### Scenario: 成功蓋章後清除 snapshots

- **WHEN** review stamp 成功且 snapshots 存在
- **THEN** canonical review 工单与章依 review-station 契约更新，snapshots 被清除、baseline 保留

#### Scenario: 子模块在验证轮未再变动时不重复进入补丁

- **WHEN** Round 1 discovery 快照已冻结脏子模块 `frontend` 与文本文件 `src/lib.rs`，修复仅修改 `src/lib.rs` 而未动 `frontend` 内部任何文件，执行 Round 2 `speclink review scope <change> --json`
- **THEN** exit code 为 0，`phase="validation"`、`state="resolved"`，`paths` 与 `files` 仅含 `src/lib.rs`，不含未变动的 `frontend`

#### Scenario: 子模块在验证轮再次修改脏文件时进入验证补丁

- **WHEN** Round 1 discovery 快照已冻结脏子模块 `frontend`，Round 1 finding 点名 `frontend`（或未点名但相邻改动），修复在 `frontend` 子模块工作区继续修改未提交内容（HEAD 未变）后执行 Round 2 `speclink review scope <change> --json`
- **THEN** exit code 为 0，`phase="validation"`、`state="resolved"`，`files` 含 `frontend` 且 `kind="submodule"`、`hunks=[]`、`beforeHash` 等于 Round 1 冻结的 `afterHash`、`afterHash` 为新脏态哈希且与 `beforeHash` 不同、`attribution` 为 `"finding"`（点名时）或 `"adjacent"`（未点名时）


<!-- @trace
source: support-gitlink-in-scope
updated: 2026-11-28T13:50:37+08:00
-->

---
### Requirement: remote workspace 使用同一 host resolver

Remote Store workspace 的 review prepare 與 review scope SHALL 在執行 agent 持有的 local checkout 操作 baseline 與 snapshots；touched 認領 SHALL 自 Store 的 evidence 記錄讀取——fs 模式讀本地 evidence 檔，remote 模式 SHALL 經 typed remote client 的 change evidence 讀取端點取得，SHALL NOT 以本地檔案替代。多 actor 的 evidence entries 其 touched SHALL 取聯集；evidence 內的 head commit SHALL NOT 參與 scope 解析。併行認領守門所需的其他 active change evidence SHALL 同樣自 Store 讀取，無 evidence 的 change 視為零認領。active changes 與 ticket SHALL 透過 typed remote client 讀取。Server SHALL NOT 新增 Git diff endpoint，也 SHALL NOT 保存 host-local baseline／snapshot。

evidence 缺席或 touched 為空時 SHALL 維持 EmptyTouched fail-closed 與 needsInput 手動路徑。離線、認證失效或 remote read 錯誤（含 evidence 讀取失敗）時 command SHALL 非零結束且不寫 baseline／snapshot，SHALL NOT 把讀取失敗靜默降級為空認領。成功 scope 後的 review add-round／stamp／discard SHALL 繼續走既有的遠端文件與 revision 契約；scope 本地 Git 錯誤 SHALL NOT 被包裝成 revision conflict。

#### Scenario: remote scope 仍使用 local checkout

- **WHEN** workspace 連線 Remote Store，server 的 change evidence 帶 touched files，local checkout 有可信 baseline
- **THEN** review scope 自 server evidence 取得 touched 認領，使用 local Git 產生與 fs mode 同欄位的 resolved payload，server 不收到 patch 或 snapshot

#### Scenario: remote 離線時零 sidecar effects

- **WHEN** review scope 取得 remote ticket 前發現連線離線
- **THEN** command 非零結束、stderr 回報連線錯誤，baseline 與 snapshots 內容不變

#### Scenario: remote task done 後 scope 自動解析

- **WHEN** remote workspace 的 change 以 task done 回報 touched files，隨後同一 checkout 執行 review scope 且未帶任何手動旗標
- **THEN** scope 回 resolved payload，touched 認領即 task done 回報的檔案集合，不出現 needsInput

#### Scenario: 多 actor evidence 取聯集

- **WHEN** 同一 change 的 evidence 含兩位 actor 的 entries，touched files 分別為不同檔案集合
- **THEN** scope 的 touched 認領為兩集合的聯集，與 fs 模式 TouchedRecord 語意一致

#### Scenario: remote evidence 缺席維持 fail-closed

- **WHEN** remote workspace 的 change 無任何 evidence 記錄，執行 review scope 且未帶手動旗標
- **THEN** command 以 needsInput 回報 EmptyTouched 理由，手動旗標路徑（--base 或 --candidate-hash 加 --include-hunk）維持可用

#### Scenario: 併行認領守門於 remote 生效

- **WHEN** 同 workspace 另一 active change 的 server evidence 認領了與本 change 重疊的檔案
- **THEN** scope 回報的守門結果與 fs 模式同形，重疊認領不被靜默忽略


<!-- @trace
source: retire-teamstore-and-drivers
updated: 2026-11-17T22:53:25+08:00
-->

---
### Requirement: 以可信 commit 重設 Apply baseline 並承接快照

`speclink review prepare <change> --base <rev> [--from <worktree-path>]` SHALL 不讀 stdin，並依下列順序執行；前三步任一失敗 SHALL 非零結束、stderr 說明原因，且 SHALL NOT 寫入任何檔案：

1. 以 `git rev-parse --verify <rev>^{commit}` 把 `<rev>` 解析成完整 SHA，解析不到即失敗；以 `git merge-base --is-ancestor` 確認該 SHA 是目前 HEAD 的祖先，不是即失敗。
2. 目標 checkout 已有該 change 的 baseline，且審查或驗證任一站工單未結（工單存在而未蓋章）時失敗，stderr 點名哪一站——該站的快照綁著舊基準。目標沒有 baseline 時不做此檢查。
3. 帶 `--from` 時，該路徑不存在即失敗；`--from` 未搭配 `--base` 時失敗。
4. 帶 `--from` 時，把來源 checkout 中該 change 的審查快照與驗證快照（`.speclink/review-scopes/<change>/` 下兩站各自的快照目錄）逐檔複製到目前 checkout 的同一位置；目標已有同名快照檔即略過（檔名為 patchHash digest，同名即同內容）；每個檔案以暫存檔加 rename 原子寫入。來源沒有快照目錄時不複製、不視為錯誤。來源的 baseline.json SHALL NOT 被複製。
5. 最後以原子寫入產生 baseline：baseCommit 為步驟 1 的完整 SHA、dirtyFilesAtStart 為當下的 dirty paths、capturedAt 為 UTC RFC3339、confidence 為 initial；格式與不帶旗標時相同。

成功 SHALL exit 0、stdout 為空。步驟 4 中途失敗時 SHALL 非零結束，已複製的快照留著、baseline 未寫；以同樣參數重跑 SHALL 略過已存在的快照並完成剩餘步驟。remote 模式下 baseline 與快照仍是本機 checkout 的資料，行為與本機相同。不帶旗標的既有行為與輸出 SHALL 維持不變。

#### Scenario: 合併後在主 checkout 寫入基準並承接快照

- **WHEN** 主 checkout 無該 change 的 baseline，HEAD 的父 commit 為 M，來源 worktree 有一份審查快照與一份驗證快照，執行 `speclink review prepare demo --base M --from ../wt-demo`
- **THEN** exit 0、stdout 為空；主 checkout 的 baseline 為 baseCommit=M 的完整 SHA、confidence=initial；兩份快照出現在主 checkout 對應的快照目錄；之後 `speclink review scope demo --json` 不再回報 baseline 缺失

#### Scenario: 指定的 commit 不是 HEAD 的祖先

- **WHEN** `--base` 指向一個不在目前 HEAD 歷史上的 commit
- **THEN** 非零結束、stderr 說明該 commit 不是 HEAD 的祖先，baseline 與快照目錄皆不變

#### Scenario: 解析不到的 rev

- **WHEN** `--base no-such-ref`
- **THEN** 非零結束、stderr 說明解析不到 commit，零寫入

#### Scenario: 有未結工單時拒絕改寫既有基準

- **WHEN** worktree 內已有該 change 的 baseline，且審查工單存在而未蓋章，執行 `speclink review prepare demo --base T`
- **THEN** 非零結束、stderr 點名審查站工單未結，baseline 位元級不變

#### Scenario: 來源沒有快照照常成功

- **WHEN** 來源 worktree 從未跑過品質關卡，執行 `speclink review prepare demo --base M --from ../wt-demo`
- **THEN** exit 0，只寫入 baseline，快照目錄不建立

#### Scenario: --from 單獨使用被拒

- **WHEN** 執行 `speclink review prepare demo --from ../wt-demo`（未帶 --base）
- **THEN** 非零結束、stderr 說明 --from 必須搭配 --base，零寫入

#### Scenario: 複製中途失敗後重跑完成

- **WHEN** 承接快照時第二個檔案寫入失敗，之後以同樣參數重跑
- **THEN** 第一次非零結束且 baseline 未寫；重跑略過已存在的第一個檔案、複製其餘檔案並寫入 baseline，exit 0

<!-- @trace
source: quality-gate-speedup
updated: 2026-10-14T21:31:03+08:00
-->