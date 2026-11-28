## MODIFIED Requirements

### Requirement: 驗證指紋錨與失效判定

<!-- BEFORE: 仅将普通文件纳入 verified_scope 指纹与失效判定，gitlink 160000 目录被判为不存在或不可读 -->

盖章时系统 SHALL 以验证工单各轮 Scope 联集记录 `{ path, hash }` 至 `verified_scope`，路径规范化、存在性判定与指纹分流规则（可以 UTF-8 读取的普通文件为行尾 CRLF→LF 规范化后的 SHA-256；无法以 UTF-8 读取的普通文件为原始位元组的 SHA-256；tree mode 160000 的 gitlink 路径视为存在并以规范 gitlink 状态字节计算 SHA-256）SHALL 与审查站位元级同构（共用同一份共享读取与指纹实现）。失效判定同构：当前全任务总数不再等于盖章时的 `verified_tasks_total`、或任一写码任务未完成、或任一 scope 条目内容不符（含缺档、gitlink 移除、子模块 commit 推进或工作区脏态变更）→ stale；补勾或取消勾 `[M]` 任务 SHALL NOT 影响判定。判定结果 SHALL NOT 以 CLI 专属查询栏位曝光；封存守门与桌面看板 SHALL 消费同一判定（不含 gitlink 的仓库输出逐位元保持不变）。

#### Scenario: 蓋章後修改範圍檔

- **WHEN** 验证盖章成功后修改任一 scope 档内容
- **THEN** 失效判定为 stale

#### Scenario: 非 UTF-8 scope 檔可蓋章

- **WHEN** 验证工单 Scope 含一个存在的非 UTF-8 档，其余守门条件皆过，执行 `verify stamp`
- **THEN** 盖章成功，`verified_scope` 记该档原始位元组的 SHA-256

#### Scenario: 蓋章後補勾手動任務不失效

- **WHEN** 写码任务全完成、一个 `[M]` 任务未勾时验证盖章成功，之后将该 `[M]` 任务勾选
- **THEN** 失效判定仍为 fresh

#### Scenario: 行尾差異不觸發失效

- **WHEN** scope 档内容仅行尾由 LF 变为 CRLF
- **THEN** 失效判定仍为 fresh

#### Scenario: 含子模块路径的验证工单可正常盖章

- **WHEN** 写码任务全完成、验证工单末轮零必修 findings 且 Scope 含 160000 gitlink `frontend`，执行 `speclink verify stamp <change>`（不读 stdin）
- **THEN** exit code 为 0，`.meta.yaml` 的 `verified_scope` 记录 `frontend` 的规范状态字节 SHA-256 且与同状态下审查站计算的指纹逐位元一致，`verify.md` 被删除

#### Scenario: 验证盖章后子模块状态变动触发失效

- **WHEN** 含子模块 `frontend` 的验证章盖下后，子模块工作区未提交内容变动或子模块 commit 推进
- **THEN** 失效判定变为 stale，执行 `speclink archive <change>` 非零拒绝并在 stderr 点名验证章下 `frontend` 盖章后已变动
