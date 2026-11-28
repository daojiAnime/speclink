## MODIFIED Requirements

### Requirement: 內容指紋錨與失效判定

<!-- BEFORE: 仅将普通文件视为存在于工作树并以文件系统直读计算指纹，gitlink 160000 目录在盖章时被判为不存在或不可读，失效判定无法追踪子模块指针与工作区脏态变化 -->

盖章时系统 SHALL 以工单各轮 Scope 的联集为范围，逐项记录 `{ path, hash }` 至 `reviewed_scope`：path 为 repo-root 相对且以 `/` 分隔（Windows 路径规范化后写入）；hash 依条目类型分流——可以 UTF-8 读取的普通文件为行尾 CRLF→LF 规范化后的 SHA-256（与既有章逐位元组相容），无法以 UTF-8 读取的普通文件为原始位元组的 SHA-256（不做行尾规范化），tree mode 160000 的 gitlink 路径视为存在于工作树并以规范 gitlink 状态字节（已初始化且干净为 `Subproject commit <head>\n`，已初始化且脏为 `Subproject commit <head>-dirty\ndirty-worktree: sha256:<digest>\n`，未初始化为父仓索引记录的 `Subproject commit <index>\n`）经同一指紋入口计算 SHA-256。联集中已不存在于工作树的普通文件或已移除的 gitlink（修正过程删除或改名）SHALL 排除于指纹之外、不入 `reviewed_scope`，盖章不因死档而失败；联集全数不存在时 SHALL 拒绝盖章（exit code 非零并列出档案）；存在但读取发生 I/O 错误的条目（含普通目录非 160000 的路径或损坏的子模块检出）SHALL 仍使盖章失败（非 UTF-8 内容不是读取失败，走位元组指纹）。remote 模式下工作树持有者 SHALL 使用同一存在性与规范字节读取语义区分存在条目与 `missing` 清单（存在的 gitlink 归入存在条目并上送指纹，不误标为 `missing`），server SHALL 沿用既有「提交指纹的 path 集合 ∪ missing = 工单联集且两者不相交」校验，离线、认证失效与 revision conflict 行为维持不变。失效判定 SHALL 为：当前全任务总数不再等于盖章时的 `reviewed_tasks_total`、或任一写码任务未完成、或任一 scope 条目内容杂凑不符（含档案或 gitlink 已不存在、gitlink commit 推进或子模块工作区脏态变更）→ 该章判为过期（stale）；全部相符 → 有效（fresh）。补勾或取消勾 `[M]` 任务 SHALL NOT 影响判定。判定结果 SHALL NOT 以 CLI 专属查询栏位曝光；封存守门与桌面本机/remote 看板 SHALL 消费同一 gitlink 读取语义（不含 gitlink 的仓库行为与输出逐位元保持不变）。

#### Scenario: 修正刪除早輪 scope 檔後仍可蓋章

- **WHEN** 工单 Round 1 的 Scope 含档案 A 与 B，修正过程删除 B 后写码任务全完成且末轮零 findings，执行 `review stamp`
- **THEN** 盖章成功，`reviewed_scope` 仅含 A 的指纹，不含 B

#### Scenario: 聯集全數消失時拒絕蓋章

- **WHEN** 工单各轮 Scope 的所有档案皆已不存在于工作树，执行 `review stamp`
- **THEN** exit code 非零，stderr 列出已消失的档案并指引还原或 `review discard`

#### Scenario: 非 UTF-8 scope 檔可蓋章

- **WHEN** 工单 Scope 联集含一个存在的非 UTF-8 档，其余守门条件皆过，执行 `review stamp`
- **THEN** 盖章成功，`reviewed_scope` 记该档原始位元组的 SHA-256

#### Scenario: 蓋章後修改範圍檔

- **WHEN** 盖章成功后修改任一 scope 档的内容
- **THEN** 失效判定为 stale

#### Scenario: 蓋章後 binary 內容變動失效

- **WHEN** 含非 UTF-8 scope 档盖章成功后，该档的位元组内容变动
- **THEN** 失效判定为 stale

#### Scenario: 蓋章後補勾手動任務不失效

- **WHEN** 写码任务全完成、一个 `[M]` 任务未勾时盖章成功，之后将该 `[M]` 任务勾选
- **THEN** 失效判定仍为 fresh

#### Scenario: 蓋章後新增任務失效

- **WHEN** 盖章成功后 tasks.md 新增一个任务（总数改变）
- **THEN** 失效判定为 stale

#### Scenario: 行尾差異不觸發失效

- **WHEN** scope 档内容仅行尾由 LF 变为 CRLF
- **THEN** 失效判定仍为 fresh

#### Scenario: 含子模块路径的工单可正常盖章

- **WHEN** 写码任务全完成、审查工单末轮零必修 findings 且 Scope 仅含一个存在的 160000 gitlink `frontend`（干净或脏），执行 `speclink review stamp <change>`（不读 stdin）
- **THEN** exit code 为 0，`.meta.yaml` 写入 `reviewed_scope` 含 `path: frontend` 与其规范状态字节的 SHA-256，`review.md` 被删除，后续未改动子模块时失效判定保持 fresh

#### Scenario: 盖章后子模块内部继续修改脏文件触发失效

- **WHEN** 含脏子模块 `frontend` 的审查章盖下后，开发者在 `frontend` 子模块工作区继续修改或新增未提交文件（子模块 HEAD 未变）
- **THEN** 失效判定变为 stale，执行 `speclink archive <change>` 非零拒绝并在 stderr 点名 `frontend` 盖章后已变动

#### Scenario: 盖章后子模块指针 commit 推进触发失效

- **WHEN** 含干净子模块 `frontend` 的审查章盖下后，子模块提交并推进至新 commit
- **THEN** 失效判定变为 stale，封存守门与桌面看板均将其识别为过期

#### Scenario: remote 模式下子模块路径正确归类为存在而非 missing

- **WHEN** remote 模式下的审查工单 Scope 含 160000 gitlink `frontend`，在本地 checkout 执行 `speclink review stamp <change>`
- **THEN** CLI 将 `frontend` 归类为存在条目并计算规范 gitlink 指纹随请求送出（不列为 `missing`），server 校验集合相等并原子落章，exit code 为 0

##### Example: 指紋比對

- **GIVEN** `reviewed_scope` 含 `{ path: "crates/a/src/lib.rs", hash: H1 }` 且该档现值杂凑为 H1
- **WHEN** 该档追加一行后重新判定
- **THEN** 现值杂凑不为 H1，判定 stale

##### Example: 指紋分流表

| 条目类型 | hash 规则 |
| --- | --- |
| UTF-8 文字（LF） | CRLF→LF 规范化后 SHA-256 |
| UTF-8 文字（CRLF） | 与同内容 LF 版杂凑相同 |
| 非 UTF-8 位元组（如 PNG） | 原始位元组 SHA-256 |
| gitlink 160000（干净或未初始化） | `Subproject commit <sha>\n` 规范字节 SHA-256 |
| gitlink 160000（工作区脏） | `Subproject commit <sha>-dirty\ndirty-worktree: sha256:<digest>\n` 规范字节 SHA-256 |
