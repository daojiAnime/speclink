## Why

在包含 git submodule（tree mode 160000 gitlink）的父仓中，只要子模块指针移动或子模块工作区有未提交改动，执行质量关卡范围冻结（`review scope` / `verify scope`）就会失败：resolver 对所有变更路径统一调用 git 读取基线对象内容并直接用文件系统读取当前路径，而 gitlink 对象既不在父仓对象库中（触发 bad object），工作区对应路径又是目录而非普通文件（触发读取目录失败）。结果候选补丁计算整体失败，`--json` 返回 candidateHash 为 null、files 为空，CLI 提示的三条处置路径（`--base`、`--candidate-hash` 加 `--include-hunk`、独立 worktree）全部无法解开。

面向与 AI Agent 协作的开发者（以及团队中追踪变更质量状态的 PO/PM），本次变更在 `/speclink-review`、`/speclink-verify`、`/speclink-quality` 与 `/speclink-archive` 阶段补齐 gitlink 160000 的不透明指针支持，使含子模块的仓库能够完整走通 discovery、validation、盖章与失效判定。

既有规格扫描记录：已检查 `change-diff-scope`、`review-station`、`verify-station`、`worktree-overlay` 与 `drift-computation`。`worktree-overlay` 已区分 submodule 检出与 linked worktree，无需改动；`drift-computation` 不冻结补丁，无需改动；本次修改 `change-diff-scope`、`review-station` 与 `verify-station` 三份规格。

## What Changes

- **不透明 gitlink 指针模型（`change-diff-scope`）**：在 discovery 与 validation 范围解析中识别 mode 160000 的 gitlink 条目，新增 `DeltaKind::Submodule`（`--json` 序列化为 `"submodule"`，`hunks` 恒为空数组，与 `binary` 同属不可按 hunk 钉选的非文本 delta）。禁止对 gitlink 路径调用父仓对象内容读取或直接把目录当普通文件读取。
- **确定性 gitlink 状态指纹（`speclink-core` 共享读取器）**：提供统一的本地工作区范围条目读取与存在性判定入口，普通文件保持原有直接读字节快路径；遇到 gitlink 目录时，按状态生成规范字节表示——干净子模块为指向 commit SHA，脏子模块追加工作区确定性摘要，未初始化子模块回退父仓索引记录的 160000 SHA——确保修改子模块内部脏文件或移动指针 commit 时都能触发 validation 复验与盖章失效判定。
- **歧义提示修正（`speclink-cli`）**：当候选集包含 `submodule` delta 且进入 `needsInput` 时，不再因 bad object 崩溃而丢失 `candidateHash` 与其余文本文件的可选 hunks；同时在 stderr 中说明 `submodule` delta 无可选文本 hunk，并在 `--include-hunk` 拒绝信息中一并涵盖 `submodule`。
- **双站盖章、失效守门与桌面端一致性（`review-station`、`verify-station`、`apps/desktop`）**：`review stamp`、`verify stamp`（本机与 remote CLI）、封存失效守门、桌面本机看板以及桌面 remote 看板的提交比对逻辑统一复用同一 gitlink 读取语义，避免把 gitlink 目录误判为「已删除文件」或「不可读文件」。
- **CLI 命令面**：沿用既有子命令与旗标，不新增子命令、不新增旗标、不改变 stdin 与成功/失败 exit code 约定（涉及子命令：`review scope`、`verify scope`、`review stamp`、`verify stamp`、`archive`）。不修改配置字段（`speclink/config.yaml` / `.speclink.yaml`），不修改生成技能模板或工具适配（claude / codex / agy 等）。

## Compatibility Impact

- **Human 输出**：不含 gitlink 的仓库输出逐位元保持不变，现有 golden 与 CLI 回归测试不受影响；仅当 `needsInput` 候选集包含 `submodule` delta 时，stderr 追加一行说明 submodule 不可按文本 hunk 钉选。`--include-hunk` 未知 ID 报错文案由仅提 binary 扩展为同时提及 binary 与 submodule。
- **`--json` 输出**：`review scope --json` 与 `verify scope --json` 的顶层字段与 `files[*]` 字段集合不变，仅 `files[*].kind` 枚举新增 `"submodule"`（additive，原有 `"modified"` / `"added"` / `"deleted"` / `"renamed"` / `"binary"` 保持不变）。`review stamp` / `verify stamp` / `list --json` / `show --json` 字段形状不变。
- **既有用户迁移**：无需迁移。旧版 snapshot 与 `.meta.yaml` 中不含 gitlink 条目者完全兼容；此前因 gitlink 失败而未冻结 snapshot 的变更，升级后可直接重跑 `review scope` / `verify scope`。

## Topologies

- **Local repo**：完全可用（`review scope`、`verify scope`、`review stamp`、`verify stamp`、`archive` 与桌面本机看板共享同一 gitlink 状态读取与解析逻辑）。
- **Remote-connected checkout**：完全可用（`review scope` / `verify scope` 与 `review stamp` / `verify stamp` 在用户本地 checkout 计算 gitlink 指纹，server 仅按既有协议接收与校验 `{ path, hash }` 集合，不新增 Git endpoint；桌面 remote 看板的提交比对同步支持 160000 gitlink）。
- **Embedded host without local spec files**：不适用（需要本地 checkout 的 `review scope` / `verify scope` 在宿主入口既定返回 `invalid_argv`；盖章接口仅校验调用端提交的 `{ path, hash }`，不读本地文件系统）。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `change-diff-scope`: 扩展 discovery patch 解析、needsInput 候选与提示、frozen snapshot 与 validation 复验以将 gitlink 160000 视为不透明指针（`kind: "submodule"`、`hunks: []`），避免对 gitlink 调用父仓对象内容读取或目录直读。
- `review-station`: 扩展内容指纹锚与失效判定规则，使 gitlink 160000 路径在盖章时视为存在并以规范 gitlink 状态字节计算指纹，且子模块 commit 移动或工作区脏态变动皆触发 stale。
- `verify-station`: 同步扩展验证指纹锚与失效判定规则，与审查站共用同一份 gitlink 读取与指纹实现，维持两站位元级同构。

## Impact

- Affected specs: `change-diff-scope`, `review-station`, `verify-station`
- Affected crates and apps: `speclink-core`, `speclink-host`, `speclink-cli`, `apps/desktop/core`, `apps/desktop/src-tauri`
- Affected code:
  - Modified: `crates/engine/speclink-core/src/util.rs`
  - Modified: `crates/engine/speclink-core/src/lifecycle/archive.rs`
  - Modified: `crates/host/speclink-host/src/change_diff.rs`
  - Modified: `crates/adapters/speclink-cli/src/verbs/station.rs`
  - Modified: `apps/desktop/core/src/query.rs`
  - Modified: `apps/desktop/src-tauri/src/remote.rs`
  - Modified: `crates/adapters/speclink-cli/tests/it/review_verbs.rs`
  - Modified: `crates/adapters/speclink-cli/tests/it/verify_verbs.rs`
