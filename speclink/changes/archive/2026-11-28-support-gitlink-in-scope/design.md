## Context

在父仓挂载 git submodule（tree mode 160000 gitlink）的项目中，子模块状态变动（指针 commit 推进或子模块工作区未提交修改）会被父仓的脏文件扫描收集进 touched 路径与全工作区差异。当前范围冻结链路在 `speclink-host` 的 `change_diff` 模块中假设每个非删除的差异条目都对应父仓对象库中的一个 blob 与工作区中的一个普通文件：

1. `parse_segment` 与 `discovery_texts` 对每个旧路径调用父仓 git 对象内容读取，但 160000 gitlink 指向的子模块 commit 不存在于父仓对象库，触发 bad object 错误。
2. `parse_segment`、`write_scope_snapshot`、`resolve_validation_scope` 与 `remediation_segment` 对每个新路径直接执行文件系统文件读取，而子模块路径是目录，触发目录不可读或被静默跳过。
3. 候选构建抛错被 `resolve_scope` 收敛为 `BaseUnresolvable` 并清空候选，导致 `candidateHash` 为 null、`files` 为空，CLI 给出的 `--base`、`--candidate-hash` + `--include-hunk`、隔离 worktree 三条处置均失效。
4. 质量站盖章与失效判定（CLI 本机/remote 盖章、封存失效守门、桌面本机与 remote 看板）均通过普通文件存在性判定与普通文件字节读取检查 scope 路径，导致 gitlink 目录被误判为已删除或不可读。

现有架构约束：

- `speclink-core::station` 的纯域算法（`stamp`、`fingerprint_scope`、`freshness`、`stale_reason`）不直接调用 git，仅通过调用端注入的 `ScopeReadFn` 与 `file_exists` 闭包读取内容。
- 本地 checkout 与 git 的共享底层探测原语统一收口在 `speclink-core::util`（如 `read_scope_bytes`、`main_git_dir`、`is_linked_worktree`），由 `speclink-core` 流程层、`speclink-host`、`speclink-cli`、`speclink-desktop-core` 与 `apps/desktop/src-tauri` 共享。
- 范围解析的唯一实现在 `speclink-host::change_diff`，本地与 remote CLI 共用同一解析器，Server 不持有工作区也不提供 Git endpoint。

## Goals / Non-Goals

**Goals:**

- 在 discovery、validation、snapshot、盖章与失效判定全链路中将 gitlink 160000 作为不透明指针一等支持，不再因 bad object 或读取目录失败阻断流程。
- 区分「子模块指针 commit 变动」与「子模块工作区脏态变动」，使两者都能生成可重现的 canonical 哈希并在 validation 复验与 stamp freshness 中被可靠侦测。
- 保持单一实现点与存储解耦：`speclink-core::util` 提供唯一的 checkout 范围条目读取器，`speclink-host::change_diff` 负责唯一范围解析，Server 与 StorePort 零协议变更。
- 维持不含 gitlink 仓库的零性能退化与零输出漂移（人类输出、JSON 输出与 golden 快照逐位元兼容）。

**Non-Goals:**

- 不递归展开子模块内部源码文件进入父仓 patch、touched 列表或 finding 路径（父仓只把子模块视为不透明指针，前端源码 hunk 审查如需文件级粒度应在子模块自身仓库执行）。
- 不支持嵌套多层子模块（子模块内部再嵌子模块仅经子模块自身 git status 体现于脏态摘要，不逐层建模）。
- 不修改 `speclink/config.yaml`、`.speclink.yaml`、生成技能文本、Server HTTP 协议或 `@speclink/engine` Node SDK 导出接口。
- 不修改 `worktree-overlay`（submodule 检出与 linked worktree 的区分已由既有变更处理）与 `drift-computation`。

## Decisions

### D1: 不透明 gitlink 指针模型与 DeltaKind::Submodule 序列化契约

在 `crates/host/speclink-host/src/change_diff.rs` 中将 gitlink 160000 建模为新的 `DeltaKind::Submodule` 枚举成员（通过既有 `#[serde(rename_all = "lowercase")]` 序列化为 `"submodule"`），`hunks` 恒为空数组，`discovery_texts` 与 `DeltaKind::Binary` 一样跳过保存文本，仅保留 `beforeHash` / `afterHash`。

- **为什么选择此方案**：与 `DeltaKind::Binary` 完全对称，天然复用 `change_diff` 已有的「无文本、靠哈希锚定」的 discovery / snapshot / validation 路径；同时显式区分 `"submodule"` 与 `"binary"`，让调用方与人类错误提示能够准确告知这是子模块指针而非二进制文件。`Snapshot` 仍维持 `SNAPSHOT_VERSION = 1`，既有不含 `"submodule"` 的快照与 JSON 消费端完全向后兼容。
- **被否决的替代方案**：
  - 递归展开子模块内部源码文件：升级上游子模块指针时会把数百个上游文件灌入父仓审查补丁，且会牵动 `git_changed_files`、finding 路径与工作区边界，违背 KISS 与 YAGNI。
  - 复用 `DeltaKind::Modified` 并保留 `Subproject commit` 文本 hunk：会把父仓合成的 1 行伪文本送入 `no_index_hunks` 对比磁盘目录，仍需到处特判且语义误导 reviewer 以为存在可钉选的源码 hunk。
  - 复用 `DeltaKind::Binary`：丢失 gitlink 语义，validation 复验无法区分读取二进制文件与读取 gitlink 状态，且错误提示误称子模块为 binary。

### D2: 共享的 gitlink 状态规范字节与单一读取实现点

在 `crates/engine/speclink-core/src/util.rs` 新增四个供全仓复用的 checkout 探测与规范字节函数，作为 local CLI、remote CLI、archive 守门、Host resolver 与桌面端唯一的 gitlink 读取实现点（并移除已被其全面取代而无调用方的 `read_bytes_opt`）：

1. `gitlink_canonical_bytes(sha: &str) -> Vec<u8>`：构造干净/未初始化 gitlink 的规范字节 `Subproject commit <sha>\n`，由 `read_scope_bytes`、`change_diff` 与桌面 remote 看板复用。
2. `gitlink_commit_at(root: &Path, rev: &str, rel_path: &str) -> Option<String>`：在 `root` 下运行 `ls-tree -z` 检查 `<rev>` 树中 `<rel_path>` 的模式是否为 `160000`，若是则返回 40 位十六进制 commit SHA。用于获取 gitlink 的基线指针与桌面 remote 看板的 `still_at_commit` 校验。
3. `read_scope_bytes(root: &Path, rel_path: &str) -> Option<Vec<u8>>`：
   - **普通文件快路径**：若 `root.join(rel_path)` 不是目录，直接走原有 `std::fs::read`（零额外 git 子进程开销）。
   - **gitlink 分支**：若目标是目录，通过父仓 `ls-files --stage -z` 确认其索引模式为 `160000` 并取得索引记录的 `<index_sha>`；非 `160000` 的普通目录直接返回 `None`（维持原有 fail-closed 行为）。
   - **已初始化子模块**（子模块目录内存在 `.git` 文件、目录或符号链接）：先以 `GIT_CEILING_DIRECTORIES` 与 `rev-parse --show-toplevel` 确认该目录是其自有顶层 git 工作树（防止空/损坏 `.git` 目录或悬空符号链接向上爬升命中父仓 `.git`），否则返回 `None`；有效时读取子模块当前 `HEAD`（若为空仓库回退 `<index_sha>`），并以 `core.quotepath=false` 执行子模块 `status --porcelain -uall -z`：
     - 工作区干净时，规范字节为 `Subproject commit <head_sha>\n`（与基线侧格式逐位元一致，未改动时前后哈希严格相等）。
     - 工作区存在未提交改动时，计算确定性脏态摘要（对 `status --porcelain -uall -z` 原始输出、`diff --no-color --no-ext-diff --full-index HEAD --` 输出，以及按正斜杠路径排序的各未追踪文件相对路径与原始字节依次做 SHA-256），规范字节为 `Subproject commit <head_sha>-dirty\ndirty-worktree: sha256:<64-hex>\n`。
   - **未初始化子模块**（子模块目录无 `.git`）：返回 `Subproject commit <index_sha>\n`。
4. `scope_entry_exists(root: &Path, rel_path: &str) -> bool`：普通文件返回 `true`；目录仅在父仓索引模式为 `160000` 且路径存在时返回 `true`；其余返回 `false`（供 `change_diff` 识别 160000 gitlink 与删除/损坏判定）。

- **为什么选择此方案**：
  - 所有消费端（`change_diff.rs`、`station.rs` 本机与 remote 盖章、`archive.rs` 失效守门、`apps/desktop/core/src/query.rs`、`apps/desktop/src-tauri/src/remote.rs`）共用同一份字节语义，杜绝两套平行实现导致的哈希漂移。
  - 脏工作区摘要保证在子模块 HEAD 不变但继续修改脏文件时，`afterHash`、`dirtyFilesAtCapture` 与 `reviewed_scope`/`verified_scope` 指纹都会随之变化，从而在 validation 复验与 stamp freshness 中可靠触发变动检测。
  - `speclink-core::station` 引擎纯函数签名保持不变（仍只消费 `ScopeReadFn` 与 `file_exists`），继续朝存储解耦方向收敛：所有本地 checkout/git 事实都在调用端与 Host 侧采集，不污染 StorePort 与 Server 协议。
- **被否决的替代方案**：
  - 脏子模块只用固定字符串 `Subproject commit <head_sha>-dirty\n`：子模块内连续两次未提交修改会产生完全相同的哈希，validation 复验会漏检第二轮修复，stamp freshness 也会对后续修改误判为 fresh。
  - 把读取器放在 `speclink-host`：`speclink-core` 的 `archive.rs` 不依赖 `speclink-host`，放在 host 会迫使 core 复制第二份实现。

### D3: change_diff 解析、快照账本与 validation 复验适配

在 `crates/host/speclink-host/src/change_diff.rs` 中完成 discovery 与 validation 全路径适配：

1. **gitlink 识别（`parse_segment`）**：在段头（首个 `@@` 之前）分别识别旧侧与新侧是否为 `160000`（覆盖 `index <old>..<new> 160000`、`new file mode 160000`、`deleted file mode 160000`、`old mode 160000`、`new mode 160000`）；对省略 `index` 行的纯脏子模块段与无 mode 行的纯 rename 段，分别通过磁盘目录 `scope_entry_exists` 与 `gitlink_commit_at(root, base, old_path)` 判定。判定为 gitlink 时：
   - `kind = DeltaKind::Submodule`，`hunks = Vec::new()`。
   - `before_hash`：旧侧为 `160000` 时由 `gitlink_commit_at(root, base, p)` 经 `gitlink_canonical_bytes` 取 `sha256:` 哈希，严禁调用父仓对象内容读取；查无 160000 条目时显式报错（旧侧原为普通文件的 typechange 仍走 `git show`）。
   - `after_hash`：新侧为 `160000` 时由 `read_scope_bytes(root, p)` 计算 `sha256:` 哈希，严禁对目录调用 `std::fs::read`；读不到时显式报错。
2. **discovery 文本与快照账本（`discovery_texts`、`write_scope_snapshot`）**：
   - `discovery_texts` 跳过 `DeltaKind::Submodule`（与 `DeltaKind::Binary` 一致，不存 `beforeText`/`afterText`）。
   - `write_scope_snapshot` 生成 `dirty_files_at_capture` 时改用 `read_scope_bytes(&ws.root, &path)`，使 gitlink 路径与其实时 `after_hash` 一致入账（确保 `push_adjacent` 的账本一致性断言严格成立）。
3. **validation 复验（`resolve_validation_scope`、`remediation_segment`）**：
   - `resolve_validation_scope` 对未在最新快照出现但更早某轮已冻结的 finding 路径（含中间轮未变动的 submodule/binary finding）同样沿 `patch_hash_chain` 回走定位最近 origin 快照；对比 `dirty_files_at_capture` 现值哈希改用 `read_scope_bytes`，并在 `deletion_frozen` 中使用 `!scope_entry_exists(&ws.root, p)`，防止已删除的 gitlink 在后续轮次被重复当作 `new_dirty`。
   - `remediation_segment` 对 `frozen.kind == DeltaKind::Submodule` 走专用分支：若 `read_scope_bytes` 返回 `None` 且 `scope_entry_exists` 为 `true`（损坏 `.git`），显式报错 fail-closed；若工作树已被替换为普通/二进制文件，改为输出真实文件/binary delta；否则与 `frozen.after_hash` 比较，相同返回 `None`，不同则输出规范 gitlink 复验段并返回 `kind: DeltaKind::Submodule`、`hunks: []`、新旧哈希（删除时 `new_path = None`、`after_hash = None`），绝不调用 `no_index_hunks`。
4. **写入顺序与失败语义**：沿用既有单文件临时路径加原子 rename 机制；任何 gitlink 探测失败都在写入 snapshot 之前返回错误，不留半写入文件或孤儿快照。

### D4: CLI 歧义提示、双站盖章与桌面端统一接线

1. **CLI 歧义提示与选段报错（`crates/adapters/speclink-cli/src/verbs/station.rs`、`crates/host/speclink-host/src/change_diff.rs`）**：
   - `apply_selection` 的未知 hunk ID 错误文案更新为说明 binary 与 submodule delta 均无可选文本 hunk。
   - `scope_needs_input_message` 对不含 `DeltaKind::Submodule` 的场景保持逐字不变；当候选 `files` 包含至少一个 `DeltaKind::Submodule` 时，在三种处置之后追加明确说明：submodule 路径没有可选文本 hunk，不能通过 `--include-hunk` 钉选（可结合 `--base` 与干净起始状态自动归属，或仅用 `--include-hunk` 钉选其余文本文件）。
2. **本机与 remote 盖章（`crates/adapters/speclink-cli/src/verbs/station.rs`）**：
   - 本机 `station_fs` 的 `Stamp` 与 remote `remote_station` 的 `Stamp` 将 `read_file` 切换为 `core::util::read_scope_bytes(&ws.root, p)`，并以物理存在性（`symlink_metadata().is_ok()`，remote 侧同时校验 `ensure_repo_relative`）划分存在条目与已删除条目：普通文件与 `160000` gitlink 归入存在条目并计算规范指纹，已从工作树移除的条目归入 `gone`/`missing`，而物理存在但不可读的非 `160000` 普通目录或损坏子模块归入存在条目并由 `read_scope_bytes` 返回 `None` 触发 fail-closed 盖章失败（严格对齐 `review-station` 规格），保证本机与 remote 行为完全一致。
3. **封存守门与桌面端（`crates/engine/speclink-core/src/lifecycle/archive.rs`、`apps/desktop/core/src/query.rs`、`apps/desktop/src-tauri/src/remote.rs`）**：
   - `guard_stale_stamps` 与桌面本机看板 `query.rs` 的 `read_file` 闭包切换为 `read_scope_bytes`。
   - 桌面 remote 看板 `remote.rs` 的 `changed_since_stamp` 切换为 `read_scope_bytes`，并在 `still_at_commit` 中先通过 `gitlink_commit_at(root, commit, path)` 与 `gitlink_canonical_bytes` 处理 gitlink 路径：当目标路径在 `commit` 处是 160000 gitlink 时，将当前 `read_scope_bytes` 结果与 `Subproject commit <commit_sha>\n` 逐字节比对，而非调用会失败的 `rev-parse <commit>:<path>` 与 `hash-object`。

## Implementation Contract

**Observable Behavior:**

- **Discovery（`review scope` / `verify scope`）**：
  - 含已变更 gitlink（脏工作区、指针 commit 推进、新增、删除、重命名、未初始化）的仓库不再因 bad object 或读取目录失败而崩溃。
  - 当基线可信且无起始脏冲突/并发重叠时，exit 0 并冻结 snapshot；`--json` 输出的 `files` 含该 gitlink 条目，其 `kind` 为 `"submodule"`、`hunks` 为 `[]`、`beforeHash` 与 `afterHash` 按状态正确填充（新增 `beforeHash=null`，删除 `afterHash=null`）。
  - 当同仓存在歧义触发 `needsInput` 时，候选仍正常计算：`--json` 的 `candidateHash` 非 null、`files` 列出文本文件可选 hunks 与 `"submodule"` 条目；人类 stderr 额外提示 submodule 不可用 `--include-hunk` 钉选。
- **Validation（续轮 `review scope` / `verify scope`）**：
  - gitlink 未再变动时，不进入验证补丁（跨多轮未变动 finding 沿 `patch_hash_chain` 保持可复验）。
  - gitlink 指针 commit 推进、子模块工作区脏内容再次修改、由脏恢复干净或被删除时，进入验证补丁并按既有规则标注 `attribution`（`finding` / `adjacent` / `new`），`kind` 为 `"submodule"`、`hunks` 为 `[]`。
- **Stamp & Freshness（`review stamp` / `verify stamp` / `archive` / Desktop）**：
  - 工单 Scope 包含 gitlink 路径时，本机与 remote 盖章均将其视为存在并写入规范状态指纹，不再报「所有范围文件已消失」或「无法读取范围文件」；非 `160000` 普通目录或损坏子模块仍触发盖章失败。
  - 盖章后若子模块未变动，失效判定保持 fresh，封存放行；若子模块 commit 移动、内部新增/修改/删除未提交文件或子模块被移除，失效判定变为 stale，封存守门拒绝并点名该路径。
  - 桌面本机看板与 remote 看板对含 gitlink scope 的已盖章变更正确显示 fresh / stale。

**Interface & Data Shapes:**

- `speclink_core::util` 新增公开函数（并移除无调用方的 `read_bytes_opt`）：
  - `pub fn gitlink_canonical_bytes(sha: &str) -> Vec<u8>`
  - `pub fn gitlink_commit_at(root: &Path, rev: &str, rel_path: &str) -> Option<String>`
  - `pub fn scope_entry_exists(root: &Path, rel_path: &str) -> bool`
  - `pub fn read_scope_bytes(root: &Path, rel_path: &str) -> Option<Vec<u8>>`
- `speclink_host::change_diff::DeltaKind` 新增枚举成员 `Submodule`，`DeltaKind::as_str` 返回 `"submodule"`，JSON 序列化为 `"submodule"`。
- `review scope --json` / `verify scope --json`：顶层与 `files[*]` 字段名不变，保持 camelCase；仅 `files[*].kind` 允许值集合扩展为 `"modified" | "added" | "deleted" | "renamed" | "binary" | "submodule"`。

**Failure Modes:**

- `git` 不可用或父仓路径非有效 git 检出：普通文件仍按文件系统读取；gitlink 目录无法从索引确认 `160000` 时返回 `None` / `false`，维持既有 fail-closed。
- 已记录为 `160000` 的旧路径在 `base` 树中查不到 gitlink commit（如异常基线损坏）：`parse_segment` 返回明确错误，不静默捏造哈希。
- 对 `submodule` 路径传入 `--include-hunk` 或不存在的 hunk ID：`apply_selection` 非零拒绝，stderr 说明 binary 与 submodule 均无可选 hunk，零 snapshot 写入。
- 子模块内部包含无法读取的未追踪文件时：脏态摘要计算忽略不可读条目不静默崩溃；对已初始化但损坏的 `.git`（含坏 `.git` 文件、空 `.git` 目录或悬空符号链接）返回 `None`，由上层（含 discovery、validation 与 stamp）fail-closed 报错。

**Acceptance Criteria:**

- `crates/engine/speclink-core` 单元测试覆盖 `gitlink_commit_at`、`scope_entry_exists`、`read_scope_bytes`（普通文件、普通目录、干净子模块、脏子模块含未追踪文件、脏内容二次修改哈希变化、未初始化子模块、损坏 `.git` 文件/空目录/悬空符号链接、删除子模块）。
- `crates/host/speclink-host` 单元测试覆盖 `resolve_candidate`、`resolve_scope`（discovery、纯 gitlink 重命名、needsInput 保留 candidateHash 与文本 hunk、validation 复验子模块脏内容修改与恢复、多轮未变动 finding 回走链、损坏 `.git` fail-closed、typechange、新增/删除 gitlink）。
- `crates/adapters/speclink-cli` 集成测试（`review_verbs`、`verify_verbs` 与 `remote_verb_parity`）覆盖端到端 `review prepare` → `review/verify scope`（human 与 `--json`、`--no-color`）→ `add-round` → `stamp`（含混合 Scope 非 160000 普通目录拒盖章、双站哈希逐位元一致与 remote 模式 gitlink 归类存在）→ `archive`（含子模块二次修改触发 stale 拒封存）以及 `needsInput` 提示。
- `apps/desktop/core` 与 `apps/desktop/src-tauri` 单元测试覆盖含 gitlink scope 的本机/remote 看板 freshness（fresh → 修改子模块 → stale，以及 `still_at_commit`）。
- `crates/engine/speclink-core` golden 测试与全受影响包 `cargo test`、`scripts/*.test.mjs` 全绿。

**Scope Boundaries:**

- **In Scope**：`speclink-core::util` 共享 gitlink 读取原语、`archive.rs` 失效守门、`speclink-host::change_diff` 解析与快照/复验、`speclink-cli` 质量站动词与提示、`apps/desktop/core` 与 `apps/desktop/src-tauri` 看板 freshness/提交比对，以及上述模块的单元与集成测试。
- **Out of Scope**：子模块内部文件级展开、多层嵌套子模块递归、技能模板文案改动、Server HTTP API 或 Node SDK 契约改动、`worktree-overlay` 与 `drift-computation`。

## Risks / Trade-offs

- **[Risk] 回归测试破坏现有 human/JSON 输出或 golden 快照** → Mitigation：不含 gitlink 的仓库绝不进入 gitlink 分支；`scope_needs_input_message` 仅在候选集实际含 `DeltaKind::Submodule` 时追加提示行；变更前后必须运行 `speclink-core` 的 `render_golden::` 测试与 `speclink-cli` 既有 `review_verbs` / `verify_verbs` / `remote_verb_parity` 集成测试确认零意外漂移。
- **[Risk] 跨平台差异（Windows 路径分隔符、CRLF 行尾、git 引号转义、测试中 `file://` 子模块协议限制）** → Mitigation：所有 git 调用统一传 `-c core.quotepath=false` 与 `-z`（NUL 分隔），路径统一正斜杠规范化，规范字节固定使用 LF 行尾；集成测试在创建本地子模块 fixture 时显式传 `-c protocol.file.allow=always`（Git 2.38+ 默认禁用本地 `file://` submodule clone），确保 Windows / macOS / Linux 行为一致。
- **[Risk] 大型子模块脏工作区计算摘要耗时** → Mitigation：普通文件快路径完全不触发 git 子进程；干净子模块仅 2 次轻量 git 调用（`rev-parse HEAD` 与 `status --porcelain`）；仅当子模块工作区实际为脏时才运行子模块 `diff HEAD --` 并哈希未追踪文件，不扫描 `.git` 与被忽略产物（`--exclude-standard` 语义由 `status --porcelain -uall -z` 默认遵守）。
- **[Risk] 仅不透明指针无法在父仓 patch 中审阅子模块内部源码行** → Mitigation：这是明确确认的 KISS/YAGNI 权衡；文档与错误提示清楚界定父仓仅追踪子模块指针与脏态指纹，后续若有充分证据需要文件级展开，可在不破坏本契约的前提下以独立变更演进。
