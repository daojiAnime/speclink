## Why

使用简体中文的开发者（团队情境下包含 PO 与 PM）在通过 AI agent 执行 Speclink 的初始化、设定、提案、分析、实作与桌面项目设定时，目前无法在工作流设定中选择简体中文：引擎 `locale` 写入验证只收 `tw`、`ja`、`en`（`spec_locale` 另收 `auto`），`analyzer` 的中文弱语气词侦测只涵盖繁体字形，内置 schema 的任务短标题指引与前端任务列三层切分也只认得繁体「驗證：」、日文「検証：」与英文「Verify: 」。新增正式的简体中文语系代码 `cn`（显示名称 `Simplified Chinese (简体中文)`）并将本仓库的工作流设定改为默认使用 `cn`，同时将引擎未设定 `locale`／`spec_locale` 时的默认语言改为 `cn`（`Simplified Chinese (简体中文)`），并优先排列 `cn` 与简体「验证：」标记，完整支持简体中文优先的工作流（显式设为 `en` 仍输出 `English`）。

## What Changes

- **引擎语系代码、默认语言与显示名称（`speclink-core`、`speclink-verbs`）**：
  - `LOCALE_CODES` 扩充为 `["cn", "tw", "ja", "en"]`，`SPEC_LOCALE_CODES` 扩充为 `["cn", "tw", "ja", "en", "auto"]`。
  - `locale_display` 新增 `Some("cn") => "Simplified Chinese (简体中文)"` 并将未设定（`None`）默认改为 `"Simplified Chinese (简体中文)"`；`spec_locale_code` 保留显式 `en` 为 `Some("en")` 以正确输出 `"English"`，既有 `tw`、`ja`、`en` 映射与未知代码原样透传行为不变。
  - `speclink workflow-config set locale cn` 与 `speclink workflow-config set spec_locale cn`（含 `--dry-run`、本地／远程／内嵌宿主路径）成为合法写入；非法值错误讯息同步列出包含 `cn` 的合法代码集合，不新增或变更子指令、旗标、stdin 或 exit code 规则。
  - `speclink init` 的 `speclink/config.yaml` 范本注释示例维持注释形态，注释说明更新为 `default: Simplified Chinese` 与 `# locale: cn`，与引擎未设定默认改为 `cn` 保持一致。
  - `crates/adapters/speclink-verbs/src/render/workflow_config.rs`（及 `mod.rs` 测试）同步更新未设定时的渲染文案为 `unset (Simplified Chinese)` 与 `unset (specs in Simplified Chinese)`。
- **规格语言指示与简体弱语气词侦测（`speclink-core`）**：
  - `instructions.rs` 在解析后的规格语言为 `cn`、未设定（默认 `cn`）或小写 `zh-cn`／`zh-sg`／`zh-hans*` 时，`speclink instructions specs` 的 `locale` 回报 `Simplified Chinese (简体中文)`，并在 `instruction` 结尾的语言段落附上简体中文模糊用语提醒（`应该、可能、也许、或许、大概、考虑、尽量、待定`）；`tw` 维持原有繁体提醒，`ja` 与显式 `en` 维持不含中文提醒。
  - `analyzer.rs` 的 CJK 弱语气词清单新增简体字形 `应该`、`也许`、`或许`、`考虑`、`尽量`，与既有繁体词及共用词（`大概`、`待定`、`可能` 含 `不可能` 豁免）并列侦测，并与 `instructions.rs` 共用词表常量。
- **内置 schema 任务短标题指引与前端任务列三层切分（`speclink-core`、`packages/ui`）**：
  - `spec-driven` 与 `no-spec-change` 两份内置 `fork.schema.yaml` 的 tasks instruction 新增简体中文行 `- Simplified Chinese: <短标题>：<行为>。验证：<验证方式>`，排在繁体、日文、英文之前。
  - `packages/ui/src/docModel.ts` 的任务验证标记清单新增 `验证：`，切分优先序为 `验证：` → `驗證：` → `検証：` → `Verify: `，确保简体中文任务在 Desktop 与 UI 检视中能正确切出标题、内文与验证三层。
- **技能原稿、产物版号与黄金快照同步（`speclink-core`）**：
  - 更新 `crates/engine/speclink-core/assets/skills/config.md`，将合法代码清单改为 `cn`、`tw`、`ja`、`en`（`spec_locale` 另含 `auto`），补入 `"Simplified Chinese" -> cn` 映射示例，并同步未设定默认简体中文语义；同时更新连带受未设定默认语系影响的 `audit.md`、`baseline.md`、`discuss.md`、`improve.md`（均采英文撰写，与 `english-skill-prompts` 的零汉字规则一致）。
  - 递增 `ASSET_VERSION`，同步更新 `assets.lock`、全部工具 golden 快照与本仓库受管技能档。
- **Desktop 项目设定页下拉与提示同步（`apps/desktop`）**：
  - `apps/desktop/src/views/project-settings/useProjectSettings.ts` 的 `LOCALE_OPTIONS` 与 `SPEC_LOCALE_OPTIONS` 加入 `"cn"`（排在 `tw` 之前）。
  - `apps/desktop/src/views/project-settings/WorkflowPage.tsx` 的 `LOCALE_LABEL` 加入 `cn: "cn（简体中文）"`。
  - `apps/desktop/src/i18n/messages.ts` 的 `settings.localeHelp`、`settings.specLocaleHelp`、`settings.localeUnset`、`settings.localeInvalidHint` 与 `settings.specLocaleInvalidHint`（`zh-TW` 与 `en` 两语）同步列出 `cn` 与默认简体中文说明。
- **本仓库默认设定与双语文件同步**：
  - 将本仓库 `speclink/config.yaml` 设为 `locale: cn`、`spec_locale: auto`（与引擎未设定默认为 `cn` 保持一致）。
  - 同步更新 `docs/configuration.md`、`docs/configuration.zh-TW.md`、`docs/verb-contract.md`、`docs/verb-contract.zh-TW.md` 中的语系代码表、内置默认说明与 `task-line-shape` 范例表。

### 三種形態下的結果

- **本機 repo（filesystem 模式）**：可用。`speclink workflow-config set locale cn`、`speclink instructions specs`、`speclink analyze`、Desktop 项目设定页下拉与任务三层切分皆原生支持 `cn` 与 `验证：`。
- **連到遠端的 checkout（remote 模式）**：可用。`workflow-config set`、`instructions specs` 与 `analyze` 皆经同一个 `speclink-core` 规则执行与验证，Server 端与本地对 `cn` 的解析、写入校验与技能渲染完全一致。
- **沒有本機規格檔的內嵌宿主（Node SDK / `createHost`）**：可用。`host.dispatch` 与 `skills.render` 共用 `speclink-core` 的 `LOCALE_CODES`、`locale_display`、`instructions` 与技能资产，不依赖本地文件系统。

### 相容性影響

- **引擎默认变更（刻意变更）**：未设定 `locale` 或 `spec_locale` 时默认改为 `Simplified Chinese (简体中文)`（`cn`）；需要英文的项目显式执行 `speclink workflow-config set locale en`／`spec_locale en` 即可，既有显式设定 `tw`、`ja`、`en` 的项目行为不变。
- **人眼输出与 `--json` 输出**：所有 `--json` 栏位名称与型别不变；`workflow-config set` 收到非法语系值时的错误讯息新增 `cn` 于合法代码清单（`cn, tw, ja, en`／`cn, tw, ja, en, auto`）；`workflow-config show` 未设定时显示 `unset (Simplified Chinese)`／`unset (specs in Simplified Chinese)`；`instructions specs` 对 `spec_locale: cn`（或未设定）回报 `locale: "Simplified Chinese (简体中文)"` 与简体中文弱语气词提醒；`instructions tasks` 的 `instruction` 多一行简体中文任务格式范例。
- **技能与回归对照（刻意变更）**：因 `config.md`、`audit.md`、`baseline.md`、`discuss.md`、`improve.md` 更新与 `ASSET_VERSION` 递增，所有内置工具（`claude`、`codex`、`copilot`、`pi`、`cursor`、`agy`、`omp`）与自定义描述子的 golden 快照与 `assets.lock` 将于同批刻意更新。
- **本仓库设定**：本仓库 `speclink/config.yaml` 改为 `locale: cn`、`spec_locale: auto`，后续在本仓库建立的新提案与规格将以简体中文撰写（既有繁体中文规格的指向名称仍依规则照正式规格原文保留）。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `workflow-config`: `workflow-config set` 的 `locale` 合法代码扩充为 `cn`、`tw`、`ja`、`en`（`spec_locale` 另含 `auto`）；`instructions specs` 支持 `cn` 映射为 `Simplified Chinese (简体中文)`、将未设定默认改为 `Simplified Chinese (简体中文)` 并附简体中文模糊用语提醒。
- `change-analysis`: `ambWeakLanguage` 的 CJK 弱语气词清单新增简体中文词形（`应该`、`也许`、`或许`、`考虑`、`尽量`），维持子字串比对与「不可能」豁免。
- `config-skill`: `speclink-config` 技能文件的合法语系代码清单、未设定默认说明与映射示例纳入 `cn`（`"Simplified Chinese" -> cn`）。
- `workflow-schemas`: 内置 `spec-driven` 与 `no-spec-change` schema 的 tasks instruction 新增简体中文短标题与 `验证：` 标记形状（排在繁体之前）。
- `desktop-config`: 项目设定产出流程页的 `locale` 与 `spec_locale` 下拉合法选项集、未设定提示与无效提示纳入 `cn` 与简体中文默认。
- `desktop-app`: 任务列三层切分的验证标记清单新增简体中文 `验证：`，优先序为 `验证：` → `驗證：` → `検証：` → `Verify: `。
- `user-documentation`: 动词参考文件（`docs/verb-contract` 中英两版）的 `task-line-shape` 区段与设定文件（`docs/configuration` 中英两版）纳入 `cn` 代码、内置默认说明与 `验证：` 切分规则及例子列。

## Impact

- 规格扫描结果：扫描 `speclink/specs/` 全部既有规格，辨识出 7 份直接相关的既有规格（`workflow-config`、`change-analysis`、`config-skill`、`workflow-schemas`、`desktop-config`、`desktop-app`、`user-documentation`）。所有变更皆为对既有语系代码集、弱语气词、任务行解析与设定界面的扩展，无须新增独立 capability。
- Affected specs: workflow-config, change-analysis, config-skill, workflow-schemas, desktop-config, desktop-app, user-documentation
- 影响的技能与工具：更新 `config`、`audit`、`baseline`、`discuss`、`improve` 五份技能原稿，并因 `ASSET_VERSION` 递增而同步全部渲染技能版号，涵盖 `claude`、`codex`、`copilot`、`pi`、`cursor`、`agy`、`omp` 与自定义描述子。
- 影响的 crate 与 app：`speclink-core`、`speclink-verbs`、`speclink-cli`（测试对照）、`speclink-node`（测试对照）、`packages/ui`、`apps/desktop`，以及本仓库 `speclink/config.yaml` 与 `docs/`。
- Affected code:
  - New: (none)
  - Modified:
    - crates/engine/speclink-core/src/workspace/config.rs
    - crates/engine/speclink-core/src/workspace/config/tests.rs
    - crates/engine/speclink-core/src/workspace/instructions.rs
    - crates/engine/speclink-core/src/workspace/init.rs
    - crates/engine/speclink-core/src/workspace/schema.rs
    - crates/engine/speclink-core/src/command/workflow_config.rs
    - crates/engine/speclink-core/src/quality/analyzer.rs
    - crates/engine/speclink-core/src/quality/analyzer/tests.rs
    - crates/engine/speclink-core/assets/schema/spec-driven/fork.schema.yaml
    - crates/engine/speclink-core/assets/schema/no-spec-change/fork.schema.yaml
    - crates/engine/speclink-core/assets/skills/config.md
    - crates/engine/speclink-core/assets/skills/audit.md
    - crates/engine/speclink-core/assets/skills/baseline.md
    - crates/engine/speclink-core/assets/skills/discuss.md
    - crates/engine/speclink-core/assets/skills/improve.md
    - crates/engine/speclink-core/tests/it/render_golden.rs
    - crates/engine/speclink-core/tests/golden/assets.lock
    - crates/engine/speclink-core/tests/golden/agy.snapshot.md
    - crates/engine/speclink-core/tests/golden/claude.snapshot.md
    - crates/engine/speclink-core/tests/golden/claude-remote.snapshot.md
    - crates/engine/speclink-core/tests/golden/claude-worktree.snapshot.md
    - crates/engine/speclink-core/tests/golden/codex.snapshot.md
    - crates/engine/speclink-core/tests/golden/copilot.snapshot.md
    - crates/engine/speclink-core/tests/golden/cursor.snapshot.md
    - crates/engine/speclink-core/tests/golden/omp.snapshot.md
    - crates/engine/speclink-core/tests/golden/pi.snapshot.md
    - crates/engine/speclink-core/tests/golden/neutral-cli.snapshot.md
    - crates/engine/speclink-core/tests/golden/neutral-tool-call.snapshot.md
    - crates/engine/speclink-core/tests/golden/neutral-tool-call-remote.snapshot.md
    - crates/adapters/speclink-verbs/src/render/workflow_config.rs
    - crates/adapters/speclink-verbs/src/render/mod.rs
    - crates/adapters/speclink-cli/tests/it/config_fail_closed.rs
    - crates/adapters/speclink-cli/tests/it/workflow_config.rs
    - crates/adapters/speclink-cli/tests/it/instructions_policy.rs
    - crates/adapters/speclink-node/__test__/workflow-config-verbs.spec.ts
    - packages/ui/src/docModel.ts
    - packages/ui/src/__tests__/docModel.test.ts
    - packages/ui/src/__tests__/taskList.test.tsx
    - apps/desktop/src/views/project-settings/useProjectSettings.ts
    - apps/desktop/src/views/project-settings/WorkflowPage.tsx
    - apps/desktop/src/i18n/messages.ts
    - apps/desktop/src/__tests__/projectSettingsView.test.tsx
    - docs/configuration.md
    - docs/configuration.zh-TW.md
    - docs/verb-contract.md
    - docs/verb-contract.zh-TW.md
    - speclink/config.yaml
  - Removed: (none)
