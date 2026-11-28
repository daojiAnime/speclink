## MODIFIED Requirements

### Requirement: 引擎版號查詢面

<!-- BEFORE: speclink --version 仅输出套件版号、架构与产物层版号，格式固定为 <套件版號> (<架構>, engine <產物層版號>)，不含编译期 Git 提交与脏态信息 -->

`speclink --version`（不读取 stdin）SHALL 同时输出套件版号、架构、产物层版号，以及在从本仓库 Git 源码树编译时的编译期 Git 修订标识（8 位小写十六进制短提交哈希，当编译期存在已追踪文件的暂存或未暂存改动时附加 `-dirty` 后缀）。产物层版号 SHALL 与技能档 frontmatter 版本栏位使用的版号同源，且 SHALL NOT 随普通代码提交递增。本次变更刻意扩展人眼输出格式，在括号内 `engine <產物層版號>` 后追加修订段：

- 有已知架构且存在有效 Git 修订时 SHALL 输出 `speclink <套件版號> (<架構>, engine <產物層版號>, <8位SHA>)` 或 `speclink <套件版號> (<架構>, engine <產物層版號>, <8位SHA>-dirty)`；
- 无已知架构且存在有效 Git 修订时 SHALL 输出 `speclink <套件版號> (engine <產物層版號>, <8位SHA>)` 或 `speclink <套件版號> (engine <產物層版號>, <8位SHA>-dirty)`；
- 编译环境缺少 `git`、源码目录不在 Git 仓库中、仓库尚无提交、或 Git 仓库根目录与当前 workspace 根目录不一致（例如发行源码包被解压于其他无关 Git 仓库内）时，编译 SHALL NOT 失败，且输出 SHALL 回退为不含修订段的既有格式 `speclink <套件版號> (<架構>, engine <產物層版號>)` 或 `speclink <套件版號> (engine <產物層版號>)`。

输出 SHALL 为单行纯文本至 stdout（不含 ANSI 转义序列，在 `--no-color` 下保持逐字节一致），stderr 为空，exit code 0。该查询面不提供 `--json` 模式。

#### Scenario: --version 含引擎版號

- **WHEN** 在干净的 Git 源码树中编译产物并执行 `speclink --version`（或在 `--no-color` 下执行）
- **THEN** exit code 为 0、stderr 为空、stdout 为单行无 ANSI 文本，同时包含套件版号、架构、`engine <產物層版號>` 与与当前 `HEAD` 一致的 8 位短提交哈希，且不带 `-dirty` 后缀

#### Scenario: 脏工作区构建输出带 dirty 标记

- **WHEN** 编译前 workspace 中存在已追踪文件的未提交改动（暂存或未暂存），重新编译并执行 `speclink --version`
- **THEN** exit code 为 0、stdout 单行括号末尾为 `<8位SHA>-dirty`，且套件版号与 `engine <產物層版號>` 保持不变

#### Scenario: 非 Git 源码树构建安全回退既有格式

- **WHEN** 在不含本仓库 Git 信息的环境（无 `.git`、`git` 不可用、无提交、或被置于外部无关 Git 仓库内）中编译并执行 `speclink --version`
- **THEN** 编译成功且 `speclink --version` exit code 为 0，stdout 单行输出回退为 `speclink <套件版號> (<架構>, engine <產物層版號>)`（无已知架构时为 `speclink <套件版號> (engine <產物層版號>)`），不含逗号分隔的修订段

#### Scenario: 仅未追踪文件不触发 dirty 标记

- **WHEN** 编译前工作区除未追踪文件外无任何已追踪文件改动，编译并执行 `speclink --version`
- **THEN** exit code 为 0、stdout 单行修订段仅含 `<8位SHA>`，不附加 `-dirty` 后缀

##### Example: 版號輸出格式

| 构建场景 | 套件版号 | 架构 | 产物层版号 | Git 修订 | `speclink --version` stdout |
| --- | --- | --- | --- | --- | --- |
| 干净 Git 检出（arm64） | `0.20.0` | `arm64` | `v1.66.0` | `f63e7113` | `speclink 0.20.0 (arm64, engine v1.66.0, f63e7113)` |
| 脏 Git 检出（x64） | `0.20.0` | `x64` | `v1.66.0` | `f63e7113-dirty` | `speclink 0.20.0 (x64, engine v1.66.0, f63e7113-dirty)` |
| 干净 Git 检出（无已知架构） | `0.20.0` | 无 | `v1.66.0` | `f63e7113` | `speclink 0.20.0 (engine v1.66.0, f63e7113)` |
| 非 Git 源码包回退（arm64） | `0.20.0` | `arm64` | `v1.66.0` | 无 | `speclink 0.20.0 (arm64, engine v1.66.0)` |
| 非 Git 源码包回退（无已知架构） | `0.20.0` | 无 | `v1.66.0` | 无 | `speclink 0.20.0 (engine v1.66.0)` |
