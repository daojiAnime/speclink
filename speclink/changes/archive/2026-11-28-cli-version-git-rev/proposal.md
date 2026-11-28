## Why

在日常开发与本地重新编译部署 `speclink` CLI 时，只要变更没有修改内嵌技能模板（`ASSET_VERSION` 不递增）且尚未走 `/release` 正式发版（`Cargo.toml` 包版本号不变），`speclink --version` 输出的 `speclink 0.20.0 (arm64, engine v1.66.0)` 就会与旧版完全相同，开发者无法通过单条命令判断 PATH 上的二进制或桌面端 sidecar 是否已包含最新代码提交，也无法区分干净提交构建与含未提交改动的脏工作区构建。

面向与 AI Agent 协作的开发者（以及在团队环境中核对工具版本的 PO/PM），本次变更在本地开发构建、归档后同步部署、环境核对（比对全局 `speclink --version` 与本 checkout 的 CLI）及桌面端本地安装校验场景下，为 `speclink --version` 补充编译期注入的 Git 短提交哈希（及脏工作区标记 `-dirty`），同时保持 `ASSET_VERSION` 仅随技能模板变化递增的纪律，避免误触发已有项目的 `speclink update`。

既有规格扫描记录：已扫描并检查 `workspace-tools`、`desktop-release`、`cli-distribution`、`desktop-app` 与 `release-skill`。其中 `workspace-tools` 的「引擎版號查詢面」直接定义了 `speclink --version` 的单行输出格式契約，为本次唯一需要修改的规格；`desktop-release` 的「本機安裝的新鮮度斷言」仅要求断言 bundle 与安装版 CLI 的引擎版号一致，不锁死解析正则（只需同步放宽 `scripts/desktop/desktop-install.mjs` 的 `engineVersionOf` 解析）；`desktop-app` 的 `parseCliVersion` 仅提取括号前的包版本号，保持兼容；`cli-distribution` 与 `release-skill` 均无需修改规格。

## What Changes

- **编译期 Git 修订采集（`speclink-cli`）**：为 `crates/adapters/speclink-cli` 新增 `build.rs` 与共享解析模块 `src/git_rev.rs`，仅当 `git rev-parse --show-toplevel` 与当前 workspace 根目录一致时采集 8 位小写十六进制短提交哈希（`git rev-parse --short=8 HEAD` 取 8 位前缀）以及全仓已追踪文件脏状态（`git status --porcelain --untracked-files=no` 非空时附加 `-dirty`），通过 `cargo:rustc-env=SPECLINK_GIT_REV=...` 注入编译期环境，并输出针对 Git 元数据（`HEAD`、当前符号引用或其最近已存在祖先目录、`packed-refs`、`index`）与全仓已追踪文件（`git ls-files -z`）的 `cargo:rerun-if-changed` 指示，确保提交、切分支、暂存及工作区未暂存代码改动都能触发精确刷新。非 Git 源码包、无提交仓库、`git` 不可用或源码被解压在其他无关 Git 仓库内部时静默回退为空（不中断编译）。
- **版本输出格式扩展（`workspace-tools`）**：更新 `crates/adapters/speclink-cli/src/main.rs` 与 `workspace-tools` 规格的「引擎版號查詢面」，将 `speclink --version` 输出扩展为在括号内 `engine <產物層版號>` 之后附带编译期修订段：有架构且有修订时输出 `speclink <套件版號> (<架構>, engine <產物層版號>, <8位SHA>[-dirty])`，无架构且有修订时输出 `speclink <套件版號> (engine <產物層版號>, <8位SHA>[-dirty])`；无 Git 修订信息时回退为既有格式 `speclink <套件版號> (<架構>, engine <產物層版號>)` 或 `speclink <套件版號> (engine <產物層版號>)`。`ASSET_VERSION` 与技能 frontmatter 版本号继续完全解耦，不随代码提交递增。
- **下游解析器与文档同步（`scripts/desktop`、`apps/desktop`、`docs`）**：更新 `scripts/desktop/desktop-install.mjs` 的 `engineVersionOf` 以同时兼容旧格式（末尾为 `)`）与新格式（`engine vX.Y.Z` 后紧跟 `, <sha>[-dirty])`），补充 `scripts/desktop/desktop-install.test.mjs` 与 `apps/desktop/src/__tests__/cliInstall.test.ts` 测试用例，并同步更新 `docs/development.md`、`docs/development.zh-TW.md`、`docs/getting-started.md` 与 `docs/getting-started.zh-TW.md` 的版本示例与说明。
- **CLI 命令面**：仅涉及既有顶层旗标 `speclink --version`（与 `-V`），不新增子命令或旗标，不读取 stdin，维持单行 stdout 与 exit code 0。不修改配置文件字段（`speclink/config.yaml`、`.speclink.yaml`），不修改内嵌技能模板或工具适配（`claude`、`codex`、`agy`、`omp` 等）。

## Compatibility Impact

- **Human 输出**：在 Git 源码树中编译的二进制，`speclink --version` 单行括号末尾新增 `, <8位SHA>` 或 `, <8位SHA>-dirty`（例如 `speclink 0.20.0 (arm64, engine v1.66.0, f63e7113)`）；非 Git 源码包构建与既有输出逐字节一致。
- **`--json` 输出与回归快照**：`--version` 不提供 `--json` 模式；其余所有命令的 `--json` 与人眼输出不变。技能渲染产物与 `ASSET_VERSION` 未变动，`crates/engine/speclink-core/tests/golden/` 与 `assets.lock` 零变化。
- **现有解析器与用户迁移**：`apps/desktop/src/core/cliInstall.ts` 的 `parseCliVersion` 仅读取 `speclink ` 后的包版本号，与新格式完全兼容且不会触发 macOS/AppImage 误重部署；`scripts/desktop/desktop-install.mjs` 的 `engineVersionOf` 同步扩展以兼容带修订段的新输出与旧版输出；安装脚本测试与 Homebrew formula 仅匹配包版本号，不受影响。既有用户与已初始化项目无需迁移，也不会因新构建被误判为技能过期。

## Topologies

- **Local repo**：完全适用，本地直接编译与安装的 `speclink` CLI 即在 `--version` 中显示当前提交哈希与脏态。
- **Remote-connected checkout**：完全适用，连接远端 Server 的本地 CLI 同样通过 `--version` 标识本地二进制构建版本；不改动任何远端 HTTP 协议或 Server 握手字段。
- **Embedded host without local spec files**：不适用，`--version` 属于 `speclink-cli` 顶层 CLI 入口（见 `command-runtime` 规格，不在共享动词解析与嵌入式宿主入口范围内）。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `workspace-tools`: 扩展「引擎版號查詢面」契约，使 `speclink --version` 在括号内 `engine <產物層版號>` 后附带编译期 Git 8 位短提交哈希及 `-dirty` 标记，并在无有效 Git 仓库信息时安全回退为既有格式。

## Impact

- Affected specs: `workspace-tools`
- Affected crates and apps: `speclink-cli`, `apps/desktop` (test only), `scripts/desktop`, `docs`
- Affected code:
  - New: `crates/adapters/speclink-cli/build.rs`
  - New: `crates/adapters/speclink-cli/src/git_rev.rs`
  - Modified: `crates/adapters/speclink-cli/src/main.rs`
  - Modified: `crates/adapters/speclink-cli/tests/it/engine_version.rs`
  - Modified: `scripts/desktop/desktop-install.mjs`
  - Modified: `scripts/desktop/desktop-install.test.mjs`
  - Modified: `apps/desktop/src/__tests__/cliInstall.test.ts`
  - Modified: `docs/development.md`
  - Modified: `docs/development.zh-TW.md`
  - Modified: `docs/getting-started.md`
  - Modified: `docs/getting-started.zh-TW.md`
