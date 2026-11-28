## MODIFIED Requirements

### Requirement: Git-backed discovery scope 解析完整 worktree patch

<!-- BEFORE: 仅支持普通文本与 binary 文件，对所有非新增路径调用父仓对象内容读取并对所有非删除路径直接以文件系统读档，遇到 gitlink 160000 时报 bad object 或读取目录失败 -->

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
### Requirement: 歧義 scope 必須 fail closed 並以 hash-pinned selection 解鎖

<!-- BEFORE: 候选含 gitlink 时候选计算直接失败导致 needsInput 返回 candidateHash=null 且 files=[]，stderr 仅列出三条通用处置且 --include-hunk 报错仅提 binary -->

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
### Requirement: frozen snapshot 綁定 discovery 與 validation patch

<!-- BEFORE: dirty_files_at_capture 与 validation 仅以文件系统直读计算现值哈希，gitlink 目录被静默跳过且 remediation_segment 无法重建 submodule 复验段 -->

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
