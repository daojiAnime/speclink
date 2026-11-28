## Context

`speclink --version` 当前在 `crates/adapters/speclink-cli/src/main.rs` 中仅拼接 `CARGO_PKG_VERSION`、静态架构名（`arm64` / `x64`）与 `speclink_core::init::ASSET_VERSION`，形如 `speclink 0.20.0 (arm64, engine v1.66.0)`。按照仓库既有契约，包版本号仅在 `/release` 发版时推进，`ASSET_VERSION` 仅在内嵌技能模板渲染内容变动时推进并受 `assets.lock` 锁定。

因此，当开发者在两次正式发版之间迭代 Rust 引擎或 CLI 代码（例如 bugfix、性能或协议扩展）并重新编译部署本地 `~/.local/bin/speclink` 或桌面端 sidecar 时，`speclink --version` 的输出与旧版完全一致，无法在单行输出中识别当前二进制究竟编译自哪一个提交、工作区是否包含未提交改动。

## Goals / Non-Goals

**Goals**

- 在不破坏包版本号（`Cargo.toml`）与技能资产版本号（`ASSET_VERSION`）职责分离的前提下，让每次从 Git 源码树编译的 `speclink` CLI 在 `--version` 单行输出中自动携带 8 位 Git 短提交哈希，以及在存在已追踪文件未提交改动时附加 `-dirty` 标记。
- 确保普通 Git 仓库、Git submodule（`.git` 为指向父仓 `modules/` 的文件）与 linked worktree（`.git` 为指向 `worktrees/` 的文件）在 macOS、Linux 与 Windows 上都能正确解析 Git 修订并在提交、切换分支、暂存或修改已追踪源码时自动刷新编译期元数据。
- 确保非 Git 源码包、`git` 不可用、空仓库无提交、或源码被解压在无关父 Git 仓库内时均不中断编译，安全回退为不含修订段的既有输出格式。
- 保持所有现有消费端（桌面端 `parseCliVersion`、`scripts/desktop/desktop-install.mjs` 的 `engineVersionOf`、安装脚本与 Homebrew formula）与测试零回归。

**Non-Goals**

- 不改变 `Cargo.toml` 包版本号或 `/release` 发版判定规则。
- 不改变 `ASSET_VERSION` 的递增纪律与 `assets.lock` 锁定机制，不修改任何技能模板或 `speclink update` 过时判定逻辑。
- 不新增 CLI 子命令、旗标、环境变量配置或 `--json` 输出模式。
- 不修改远端 HTTP 协议握手字段（`engineVersion` / `skills.version`）或桌面端关于页（`AppInfo`）数据结构。

## Decisions

### D1: 将 Git 修订段置于括号末尾而非拼接在包版本号之后

**决定**：将 `speclink --version` 的单行格式扩展为：
- 有已知架构且有修订：`speclink <pkg> (<arch>, engine <engine>, <sha8>[-dirty])`
- 无已知架构且有修订：`speclink <pkg> (engine <engine>, <sha8>[-dirty])`
- 无有效 Git 修订（回退）：保持既有 `speclink <pkg> (<arch>, engine <engine>)` 或 `speclink <pkg> (engine <engine>)`

**理由**：`apps/desktop/src/core/cliInstall.ts` 的 `parseCliVersion` 使用 `/^speclink (\S+)/` 提取首个词元并与桌面 app 的 `appVersion`（`X.Y.Z`）做精确相等比对。若采用 SemVer build metadata 形式（如 `0.20.0+f63e7113`），会导致 macOS 与 Linux AppImage 启动自检持续误判 `version-mismatch` 并陷入反复重部署。将修订段放在括号末尾既保留首词元为纯包版本号，又让 `engine <ASSET_VERSION>` 保持清晰独立；仅需放宽 `scripts/desktop/desktop-install.mjs` 中 `engineVersionOf` 对 `engine` 字段右侧分隔符的匹配（接受 `,` 或 `)`）。

**备选方案**：
- 拼接为 `0.20.0+f63e7113`：破坏桌面端 `parseCliVersion` 与 `cliInstallStatus` 的版本相等判定，已淘汰。
- 新增独立 `--version --verbose` 旗标：增加 CLI 旗标面且违背「单条命令一眼辨识构建版本」的目标，已淘汰。

### D2: 在 `speclink-cli` 边界内以 `build.rs` 采集 Git 修订与脏态，并限定仓库归属

**决定**：归属边界严格留在 `crates/adapters/speclink-cli`（新增 `build.rs`、共享模块 `src/git_rev.rs` 以及 `src/main.rs` 的纯格式化函数 `format_version`），`speclink-core` 保持纯领域层，不调用 git 也不感知构建修订。

`build.rs` 复用 `src/git_rev.rs` 的具体采集与刷新规则：
1. **仓库归属防误读与 Windows 路径兼容**：由 `CARGO_MANIFEST_DIR` 向上回退三级得到 `workspace_root`，执行 `git -C <manifest_dir> -c core.quotepath=false rev-parse --show-toplevel`（调用前清除 `GIT_DIR`、`GIT_WORK_TREE`、`GIT_INDEX_FILE` 环境变量，避免 Git hook 中构建覆盖 `-C`）；仅将 `fs::canonicalize` 的结果用于比对两者是否一致，后续全部 `git -C` 与 `Path::join` 均沿用未 canonicalize 的 `workspace_root`（避免 Windows `\\?\` 逐字前缀禁用 `/` 到 `\` 规范化），并在拼接 Git 返回的 `/` 相对路径时在 Windows 上将 `/` 转为 `\`。该判定同时支持普通 clone、Git submodule 与 linked worktree，同时防止将发行源码包解压在外部无关 Git 仓库时误印外部仓库的提交哈希。
2. **短哈希与脏态定义**：以 `git -C <root> rev-parse --short=8 HEAD` 获取短提交哈希，校验至少 8 位且全为小写十六进制字符后固定取前 8 位前缀（忽略用户本机 `core.abbrev` 与前缀碰撞长度变化）；以 `git -C <root> -c core.quotepath=false status --porcelain --untracked-files=no` 判断全仓是否存在已追踪文件的暂存或未暂存改动（与 `git describe --dirty` 语义一致，不因未追踪的草稿或新建变更目录产生误报），非空时追加 `-dirty`。
3. **跨平台重建触发（`cargo:rerun-if-changed`）**：
   - 使用 `git -C <root> rev-parse --git-path <name>` 解析 `HEAD`、当前符号引用（若 `git symbolic-ref -q HEAD` 成功；松散引用尚不存在时向上查找 git 目录内最近已存在的祖先目录）、`packed-refs`（若存在）与 `index`，相对路径一律以未 canonicalize 的 `<root>` 解析后输出 `cargo:rerun-if-changed=<path>`，从而在普通仓库、submodule 与 linked worktree 下均准确监听提交、切换分支与暂存变动。
   - 同时以 `git -C <root> -c core.quotepath=false ls-files -z` 枚举全仓全部已追踪文件路径并输出 `cargo:rerun-if-changed=<abs_path>`，使与 `git status --porcelain --untracked-files=no` 范围完全对齐——任何已追踪文件的未暂存改动或还原都会触发 `build.rs` 重新判定 `-dirty`，而在零改动重复构建或测试时保持 Cargo 缓存命中、无额外重编译开销。
4. **静默回退**：上述任一步骤中 `git` 不可用、返回非零或输出不合法时，`build.rs` 以 exit 0 结束且将 `SPECLINK_GIT_REV` 设为空串，编译永不失败。

**备选方案**：
- 引入 `vergen` / `git2` 等外部 build-dependency：增加编译时间、第三方依赖面与跨平台静态链接成本；本仓库已原生调用系统 `git`，无须新增 crate 依赖，已淘汰。
- 无条件强制每次重跑 `build.rs`：会导致连续执行 `cargo test -p speclink-cli` 时每次无故重连二进制，拖慢开发测试反馈，已淘汰。

### D3: 下游引擎版号解析器兼容新旧两种输出并补齐双端测试与开发文档

**决定**：
- 更新 `scripts/desktop/desktop-install.mjs` 的 `engineVersionOf` 正则，从仅匹配行尾 `engine (\S+?)\)?\s*$` 改为匹配词边界后的引擎版号段（`/\bengine ([^\s,)]+)/`），使其同时解析旧格式 `speclink 0.1.0 (arm64, engine v1.14.0)`、无架构格式 `speclink 0.1.0 (engine v1.14.0, abcdef12)` 与新格式 `speclink 0.20.0 (arm64, engine v1.66.0, f63e7113-dirty)`，且在缺少 `engine` 段时维持抛出含 `engine` 提示的错误。
- 在 `scripts/desktop/desktop-install.test.mjs` 与 `apps/desktop/src/__tests__/cliInstall.test.ts` 中新增针对带提交哈希及 `-dirty` 标记的新格式断言。
- 同步更新 `docs/development.md`、`docs/development.zh-TW.md`、`docs/getting-started.md` 与 `docs/getting-started.zh-TW.md` 中 CLI 版本示例与「比对 PATH 与本 checkout 的 CLI 版本」说明，指出即使 `engine v…` 未变，也可直接通过提交哈希与 `-dirty` 辨识是否为本源树构建。

## Implementation Contract

- **In Scope**：
  - `crates/adapters/speclink-cli/src/git_rev.rs` 与 `crates/adapters/speclink-cli/build.rs`：编译期检测 workspace 归属的 Git 8 位小写短提交哈希与全仓已追踪脏态，导出 `SPECLINK_GIT_REV`（空字符串表示无 Git 信息）并输出精准 `cargo:rerun-if-changed` 指令；在 `git_rev.rs` 内建临时仓库隔离单测覆盖干净树、暂存/未暂存脏态、仅未追踪文件、无 `.git`、空仓库、外部父仓库、缺失 `git` 与 linked worktree 等全场景。
  - `crates/adapters/speclink-cli/src/main.rs`：新增纯函数 `format_version(pkg: &str, arch: Option<&str>, engine: &str, git_rev: Option<&str>) -> String` 并在静态 `VERSION` 中读取 `option_env!("SPECLINK_GIT_REV")`（过滤空白字符串）生成最终 `--version` 字符串；补充单元测试覆盖全部 6 种组合及 Clap `--version` 组装层输出。
  - `crates/adapters/speclink-cli/tests/it/engine_version.rs`：复用 `src/git_rev.rs`（消除逻辑重复）验证真实编译产物执行 `speclink --version` 与 `speclink --version --no-color` 均为单行无 ANSI 输出、exit code 0，且与当前 `HEAD` 及全仓已追踪工作区状态严格一致。
  - `scripts/desktop/desktop-install.mjs` 与 `scripts/desktop/desktop-install.test.mjs`：更新 `engineVersionOf` 并覆盖带 commit、带 `-dirty`、无架构与旧格式用例。
  - `apps/desktop/src/__tests__/cliInstall.test.ts`：补充 `parseCliVersion` 解析含 commit 与 `-dirty` 输出的回归测试。
  - `docs/development.md`、`docs/development.zh-TW.md`、`docs/getting-started.md` 与 `docs/getting-started.zh-TW.md`：更新示例输出与本地版本核对段落，保持英繁双语对等。
- **Out of Scope**：
  - 不修改 `crates/engine/speclink-core/src/workspace/init.rs` 的 `ASSET_VERSION`、`crates/engine/speclink-core/tests/golden/` 或 `assets.lock`。
  - 不修改 `Cargo.toml` 包版本号、`apps/desktop/src-tauri/tauri.conf.json`、`apps/desktop/src/core/cliInstall.ts` 逻辑、远端 HTTP 协议或 `/release` 技能。
- **Failure / Fallback Modes**：
  - 当编译环境无 `git`、无 `.git`、无任何 commit、或源码目录位于无关父 Git 仓库内时，`SPECLINK_GIT_REV` 为空，编译照常成功，`speclink --version` 输出退回 `speclink <pkg> (<arch>, engine <engine>)`（或无架构版本）。
  - 当 `engineVersionOf` 收到不含 `engine` 段的旧版输出（如 `speclink 0.1.0 (arm64)`）时，仍抛出明确包含 `engine` 的错误。
- **Acceptance Criteria**：
  - `cargo test -p speclink-cli --bin speclink`（单元测试 `format_version`、`cli_command` `--version` 与 `git_rev` 全场景）与 `cargo test -p speclink-cli --test it engine_version::` 全绿。
  - `cargo test -p speclink-core --test it render_golden::` 零漂移全绿。
  - `npm test -w apps/desktop` 与 `node --test scripts/*.test.mjs scripts/*/*.test.mjs`（含 `desktop-install.test.mjs`、双语对等与文档守卫）全绿。

## Risks / Trade-offs

- **全仓 `ls-files` 监听**：`cargo:rerun-if-changed` 枚举全仓数百个已追踪文件与 Git 元数据，相比仅监听 `crates/` 会多出少量 `stat` 检查 → 在本仓规模下耗时可忽略，且彻底消除了修改 `crates/` 外已追踪文件后 `SPECLINK_GIT_REV` 与全仓 `git status` 不一致的过期问题。
- **Windows 路径或非 ASCII 文件名**：`git ls-files` 默认会转义非 ASCII 路径且 `fs::canonicalize` 在 Windows 会生成 `\\?\` 前缀 → 统一使用 `-c core.quotepath=false` 与 `-z` 以原始 NUL 分隔路径读取，仅将 `canonicalize` 用于路径等价比较、以未 canonicalize 的 `workspace_root` 拼接路径并在 Windows 上将 `/` 转为 `\`，避免 `\\?\` 与分隔符失效。
- **外部脚本依赖旧版正则**：若外部使用者自行用 `/engine (\S+)\)$/` 解析 `--version` → 已在 Compatibility Impact 明确标注此刻意格式变更，并更新仓库内全部解析器与测试；括号前包版本号与括号内 `engine vX.Y.Z` 前缀均保持不变，将影响降到最低。

## Migration Plan

无需用户迁移或数据结构变更。代码合并并重新编译部署本地 `speclink` CLI 后，`speclink --version` 立即生效；如需回退，仅需回滚本变更提交即可恢复旧版格式。

## Open Questions

无未决问题。
