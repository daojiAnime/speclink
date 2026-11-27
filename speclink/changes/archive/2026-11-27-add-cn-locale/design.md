## Context

Speclink 的工作流语系政策（`locale` 与 `spec_locale`）决定 AI 产出文件、规格散文与品质关卡报告的语言，并由 `speclink-core` 作为本地 repo、远程 Server 与内嵌宿主（Node SDK / `createHost`）的唯一实作来源。目前官方合法代码仅有 `tw`、`ja`、`en`（`spec_locale` 另含 `auto`），导致使用简体中文的团队无法经 `speclink workflow-config set` 或 Desktop 项目设定页选取简体中文；同时 `analyzer` 的 CJK 弱语气词清单、`fork.schema.yaml` 的任务短标题范本以及 `@speclink/ui` 的任务列验证标记皆未涵盖简体中文词形（`应该`、`尽量`、`验证：` 等）。

## Goals / Non-Goals

**Goals:**
- 在 `speclink-core` 正式新增语系代码 `cn`（显示名称 `Simplified Chinese (简体中文)`），完整贯穿 `LOCALE_CODES`、`SPEC_LOCALE_CODES`、`locale_display`、`workflow-config set` 写入验证、`instructions specs` 语言提示与 `analyzer` 弱语气词侦测。
- 将引擎未设定 `locale`／`spec_locale` 时的默认语言切换为 `cn`（`Simplified Chinese (简体中文)`），所有代码清单与验证标记均以 `cn`／简体中文优先（显式设 `en` 维持 `English`），并将本仓库的 `speclink/config.yaml` 设为 `locale: cn`、`spec_locale: auto`。
- 补齐内置 schema（`spec-driven` 与 `no-spec-change`）的简体中文任务短标题指引与 `@speclink/ui` 的 `验证：` 任务切分标记，使简体中文任务在 Desktop 看板与详情页获得与繁体、日文、英文同等的体验。
- 同步更新受影响的技能原稿（`config.md`、`audit.md`、`baseline.md`、`discuss.md`、`improve.md`）、`speclink-verbs` 未设定渲染文案、`ASSET_VERSION`、golden 快照与 `assets.lock`、Desktop 项目设定页下拉选项与提示，以及 `docs/configuration`、`docs/verb-contract` 中英两版文件。

**Non-Goals:**
- 不新增 Desktop / `@speclink/ui` 的 `zh-CN` 完整界面 i18n 文案包（约 2,300 行，属独立议题；本次仅更新项目设定页的 `locale`／`spec_locale` 下拉选项与提示文字）。
- 不接受 `zh-CN` 或 `Simplified Chinese` 作为官方写入代码（写入端维持精简的两码风格 `cn`/`tw`/`ja`/`en`，读取端则继续维持宽容透传）。
- 不修改对话语言规则（`conversation-language.md` 继续由使用者对话语言决定聊天回复语言）。

## Decisions

### D1：引擎语系代码 `cn`、默认语系切换与显示名称映射

**选择**：
- 在 `crates/engine/speclink-core/src/workspace/config.rs`（唯一实作落点，供 CLI、Server、Node SDK、Desktop 透过稳定 Host／Command 契约共用）扩充：
  - `LOCALE_CODES: [&str; 4] = ["cn", "tw", "ja", "en"]`
  - `SPEC_LOCALE_CODES: [&str; 5] = ["cn", "tw", "ja", "en", "auto"]`
  - `locale_display`：新增 `Some("cn") => "Simplified Chinese (简体中文)".to_string()` 并将 `None` 默认改为 `"Simplified Chinese (简体中文)".to_string()`；`spec_locale_code` 不再把 `en`/`english` 折回 `None` 而是保留 `Some("en")`，使显式 `en` 维持输出 `"English"`。
- 写入验证沿用 `validate_policy_locales` 与 `check_locale_code`（大小写敏感：`CN` 仍被拒绝；空字串仍移除键回到未设定），YAML 写出沿用既有 `update_workflow_config_text` 与 `util::yaml_scalar` 跳脱，无任何平行实作。
- `crates/engine/speclink-core/src/workspace/init.rs` 的 `WORKFLOW_CONFIG_TEMPLATE` 保持 `locale` 与 `spec_locale` 为注释示例，注释更新为 `default: Simplified Chinese` 与 `# locale: cn`，与引擎未设定默认改为 `cn` 一致，并维持现有 `init 范本的政策写入位置` 契约不破坏既有测试。

**替代方案与排除理由**：
- 使用 `zh-CN` 作为写入代码：被否决——既有代码皆为两码小写（`cn`、`tw`、`ja`、`en`），混用 BCP 47 连字号风格会让 CLI 与 Desktop 选项风格不一致。
- 维持未设定默认为 `English`（仅将本仓库设为 `cn`）：被否决——使用者明确要求将引擎未设定默认语言改为 `cn`（`Simplified Chinese (简体中文)`），并保持显式 `en` 输出 `English` 以提供清晰退路。

### D2：规格语言指示与 analyzer 简体中文弱语气词侦测

**选择**：
- 在 `crates/engine/speclink-core/src/quality/analyzer.rs` 中定义 `pub(crate) const CJK_WEAK_CN_DISPLAY`（`["应该", "可能", "也许", "或许", "大概", "考虑", "尽量", "待定"]`）与 `pub(crate) const CJK_WEAK_TW_DISPLAY`（`["應該", "可能", "也許", "或許", "大概", "考慮", "盡量", "待定"]`）及繁体异体 `CJK_WEAK_TW_VARIANTS`（`["儘量"]`），由 `weak_pattern_in` 与 `instructions.rs` 共用，避免词表与提示模板各自维护而漂移。`可能`（含 `不可能` 豁免）繁简字形相同，所有匹配维持行首 `#` 跳过与每行至多报一笔 `ambWeakLanguage`（Suggestion）的既有纪律。
- 在 `crates/engine/speclink-core/src/workspace/instructions.rs` 的 `build_artifact`（`specs` 产出物）以单一英文模板分流中文模糊用语提醒：
  - 当解析后规格语言代码为 `None`（未设定，默认 `cn`）、`cn`、`zh-cn`、`zh-sg` 或以 `zh-hans` 开头（不区分大小写，涵盖 `zh-Hans-CN`、`zh-Hans-SG`）时，附加简体中文提醒：`Vague Chinese wording (应该、可能、也许、或许、大概、考虑、尽量、待定) is flagged by the analyzer just like should/may/TBD — state requirements with SHALL/MUST.`
  - 当代码为 `tw` 或其他 `zh` 开头代码时，维持既有繁体中文提醒：`Vague Chinese wording (應該、可能、也許、或許、大概、考慮、盡量、待定) is flagged by the analyzer just like should/may/TBD — state requirements with SHALL/MUST.`
  - `ja` 与显式 `en` 维持不附加中文提醒；结构标记、规范关键字与名称规则段落位置与格式不变。

**替代方案与排除理由**：
- 依项目 `spec_locale` 决定 analyzer 要扫繁体还是简体：被否决——`analyze` 是纯文件内容静态扫描且不读取 `config.yaml` 语系，同时涵盖繁简字形更简单，能直接抓到混写的弱语气词，零额外耦合。

### D3：任务短标题简体指引与前端 `验证：` 三层切分优先序

**选择**：
- 在 `crates/engine/speclink-core/assets/schema/spec-driven/fork.schema.yaml` 与 `crates/engine/speclink-core/assets/schema/no-spec-change/fork.schema.yaml` 的 tasks instruction 中，于 Traditional Chinese 之前新增一行（简体中文优先）：
  - `- Simplified Chinese: <短标题>：<行为>。验证：<验证方式>`
  - 同时更新 `crates/engine/speclink-core/src/workspace/schema.rs` 的测试断言以涵盖四组标记。
- 在 `packages/ui/src/docModel.ts`（`@speclink/ui` 共用层唯一实作落点，Desktop 详情页与封存页直接沿用既有 `TaskList` 与 `splitTaskText`，不在 `apps/*` 另写解析）将 `VERIFY_MARKS` 扩充为：
  1. `{ mark: "验证：", skipCode: false }`
  2. `{ mark: "驗證：", skipCode: false }`
  3. `{ mark: "検証：", skipCode: false }`
  4. `{ mark: "Verify: ", skipCode: true }`
  - 维持全形标记不看行内程式码、半形 `Verify: ` 与 `: ` 忽略行内程式码、前段优先以全形 `：` 切分再退回半形 `: ` 的既有规则，并于 `packages/ui/src/__tests__/docModel.test.ts` 与 `packages/ui/src/__tests__/taskList.test.tsx` 补齐简体任务切分测试。

**替代方案与排除理由**：
- 省略 `验证：` 让简体任务退回使用 `Verify: `：被否决——会导致 `locale: cn` 写出的中文任务混用英文验证标签，或因 Agent 自然写出「验证：」而使 Desktop 无法折叠验证层。

### D4：技能原稿、CLI 渲染文案、Desktop 项目设定下拉与双语文件同步

**选择**：
- **技能原稿**：更新 `crates/engine/speclink-core/assets/skills/config.md`，将合法语系代码更新为 `cn`, `tw`, `ja`, `en`（`spec_locale` 另含 `auto`），补入 `"Simplified Chinese" -> cn` 映射示例与未设定默认简体中文说明；同步更新连带描述未设定默认语系的 `audit.md`、`baseline.md`、`discuss.md`、`improve.md`（均采英文撰写，遵守 `english-skill-prompts` 的零汉字规则）；同步递增 `ASSET_VERSION`、更新 `assets.lock`、全部 12 份 golden 快照与本仓库受管技能档。
- **CLI 渲染文案**：更新 `crates/adapters/speclink-verbs/src/render/workflow_config.rs`（及 `mod.rs` 测试），使未设定时的展示文案为 `unset (Simplified Chinese)` 与 `unset (specs in Simplified Chinese)`。
- **Desktop 项目设定页**：
  - `apps/desktop/src/views/project-settings/useProjectSettings.ts`：`LOCALE_OPTIONS = ["cn", "tw", "ja", "en"]`、`SPEC_LOCALE_OPTIONS = ["auto", "cn", "tw", "ja", "en"]`（单纯扩充既有 `@speclink/ui` `SettingsChoice` 下拉选项，不另建元件）。
  - `apps/desktop/src/views/project-settings/WorkflowPage.tsx`：`LOCALE_LABEL` 补上 `cn: "cn（简体中文）"`。
  - `apps/desktop/src/i18n/messages.ts`：`settings.localeHelp`、`settings.specLocaleHelp`、`settings.localeUnset`、`settings.localeInvalidHint` 与 `settings.specLocaleInvalidHint`（`zh-TW` 与 `en`）同步反映 `cn` 与默认简体中文。
  - 补齐 `apps/desktop/src/__tests__/projectSettingsView.test.tsx` 测试验证选取与储存 `cn` 正常且无无效警示。
- **双语文件**：同步更新 `docs/configuration.md`、`docs/configuration.zh-TW.md`（语系代码表与默认语言说明）以及 `docs/verb-contract.md`、`docs/verb-contract.zh-TW.md`（`task-line-shape` 四个验证标记、优先序与四语例子表），确保 `scripts/docs/docs-parity.test.mjs` 双语结构完全对等。

### D5：本仓库工作流设定切换为 `locale: cn`、`spec_locale: auto` 与排程依赖

**选择**：
- 在本变更实作收尾阶段，通过 `speclink workflow-config set locale cn` 与 `speclink workflow-config set spec_locale auto` 将本仓库 `speclink/config.yaml` 切换为简体中文默认（`context`、`rules`、`review` 内容不受此设定变更影响）。
- 因本变更与进行中的 `add-omp-tool` 及提案中的 `english-skill-prompts` 共享 `init.rs`（`ASSET_VERSION`）、`assets/skills/config.md`、golden 快照、`assets.lock`、`speclink/config.yaml` 与 `docs/configuration`，在 landscape check 以 `speclink change depends` 宣告前置依赖 `add-omp-tool` 与 `english-skill-prompts`，避免改动冲突。

## Implementation Contract

- **Behavior**：
  - `speclink workflow-config set locale cn` 与 `speclink workflow-config set spec_locale cn` 在本地、远程与内嵌宿主皆以 exit code 0 成功写入并保留文件其余行与注释；`workflow-config set locale CN` 或显示名称仍以非零 exit code 拒绝并在 stderr 列出 `cn, tw, ja, en`（`spec_locale` 列出 `cn, tw, ja, en, auto`）。
  - `speclink instructions proposal|design|tasks|apply --change <name> --json` 在有效 `locale` 为 `cn` 或未设定时回报 `"locale": "Simplified Chinese (简体中文)"`；`speclink instructions specs --change <name> --json` 在有效 `spec_locale` 为 `cn`、未设定（或 `auto` 跟着 `locale: cn`/未设定）时回报 `"locale": "Simplified Chinese (简体中文)"`，且 `instruction` 结尾段落含 `Write spec prose in Simplified Chinese (简体中文)`、`Requirement and scenario names are prose too: write them in Simplified Chinese (简体中文)` 与简体中文模糊用语提醒（显式 `en` 回报 `"English"` 且不含中文提醒）。
  - `speclink analyze <change> --json` 在 delta spec 非标题行含 `应该`、`也许`、`或许`、`考虑`、`尽量` 时，报出 `ambWeakLanguage` Suggestion（`Vague language '<pattern>' found`）；既有繁体词形与 `不可能` 豁免不变。
  - `speclink instructions tasks --change <name> --json`（`spec-driven` 与 `no-spec-change`）的 `instruction` 列出四种任务格式（简体、繁体、日文、英文），其中简体为 `<短标题>：<行为>。验证：<验证方式>`。
  - `@speclink/ui` 的 `splitTaskText` 依 `验证：` → `驗證：` → `検証：` → `Verify: ` 优先序切分任务标题、内文与验证；Desktop 项目设定页的 `locale` 与 `spec_locale` 下拉包含 `cn（简体中文）` 且储存 `cn` 时不显示无效警示。
  - 本仓库执行 `speclink workflow-config show --json` 回报 `locale: "cn"`、`specLocale: "auto"`。
- **Interface / Data Shape**：
  - `crates/engine/speclink-core/src/workspace/config.rs`：`LOCALE_CODES` 为 `["cn", "tw", "ja", "en"]`，`SPEC_LOCALE_CODES` 为 `["cn", "tw", "ja", "en", "auto"]`，`locale_display(None | Some("cn"))` 回传 `"Simplified Chinese (简体中文)"`，`locale_display(Some("en"))` 回传 `"English"`。
  - `packages/ui/src/docModel.ts`：`splitTaskText` 函式签名与回传型别不变，内部 `VERIFY_MARKS` 将 `{ mark: "验证：", skipCode: false }` 置于首位。
  - `apps/desktop/src/views/project-settings/useProjectSettings.ts`：`LOCALE_OPTIONS` 含 `"cn"`、`SPEC_LOCALE_OPTIONS` 含 `"cn"`。
  - CLI `--json` 栏位名称、型别与所有子指令签名不变。
- **Failure Modes**：
  - `workflow-config set locale` 收到非 `cn`/`tw`/`ja`/`en`/空字串（如 `CN`、`zh-CN`、`简体中文`）时，以非零 exit code 拒绝且 stderr 列出完整合法代码，设定档零改动（`--dry-run` 时 stdout 无 diff）。
  - `speclink/config.yaml` 语法损毁时，`workflow-config set`、`workflow-config show` 与 `instructions` 维持 fail-closed，不退回默认。
  - 远程模式发生 revision 冲突、离线或认证失效时，维持既有非零 exit code 与语义错误讯息，不覆盖他人写入。
- **Acceptance Criteria**：
  - `cargo test -p speclink-core` 通过（含 `config::tests`、`instructions::tests`、`analyzer::tests`、`schema::tests`、`render_golden::`、`skill_authoring::`）。
  - `cargo test -p speclink-verbs`、`cargo test -p speclink-cli --test it workflow_config::`、`instructions_policy::` 与 `remote_verb_parity::` 通过。
  - `npm test -w @speclink/ui` 与 `npm test -w @speclink/desktop` 通过。
  - `speclink workflow-config show --json` 在本仓库回传 `locale: "cn"`、`specLocale: "auto"`。
  - `node --test scripts/*.test.mjs scripts/*/*.test.mjs` 全数通过（含中英文文件对等与连结检查）。
- **Scope Boundaries**：
  - **In scope**：`speclink-core` 语系代码／未设定默认改为 `cn`／显示名／指示／弱语气词／schema 任务范本／受影响技能原稿（`config.md`、`audit.md`、`baseline.md`、`discuss.md`、`improve.md`）／`ASSET_VERSION`／golden 与 `assets.lock`、`speclink-verbs` 未设定渲染文案、`packages/ui` `验证：` 切分、`apps/desktop` 项目设定下拉与提示、`docs/configuration` 与 `docs/verb-contract` 双语文件、本仓库 `speclink/config.yaml`。
  - **Out of scope**：Desktop / `@speclink/ui` 整套 `zh-CN` UI 翻译包、`conversation-language.md` 对话语言规则。

## Risks / Trade-offs

- **[回归对照与现有测试字串断言]** 多个 CLI／core／verbs／node／Desktop 测试断言错误讯息含 `tw, ja, en` 或默认 `English`，加上技能原稿改动会触发 `assets.lock` 与全套 golden 快照变更 → **Mitigation**：逐一更新既有断言为 `cn, tw, ja, en`／`cn, tw, ja, en, auto` 与默认 `Simplified Chinese (简体中文)`，同步递增 `ASSET_VERSION` 并以 `UPDATE_GOLDEN=1` 更新全套 golden，执行 `cargo test -p speclink-core`、`cargo test -p speclink-verbs`、`cargo test -p speclink-cli`、`npm test -w @speclink/ui`、`npm test -w @speclink/desktop` 与 `node --test scripts/*.test.mjs scripts/*/*.test.mjs` 全数验证。
- **[跨平台（Windows / macOS / Linux）字元编码与行尾]** UTF-8 简体汉字出现在 Rust 源码、YAML schema 与 Markdown 文件中，Windows 默认 CRLF 下 `render_golden` 与 `schema fork` 需一致 → **Mitigation**：沿用既有 `normalize_eol` 与 Rust UTF-8 字面，确认 `schema fork` 位元级比对与 golden 测试通过。
- **[与 `add-omp-tool`、`english-skill-prompts` 的共享文件冲突]** 三个变更皆触碰 `init.rs`（`ASSET_VERSION`）、golden 快照与 `assets.lock`，且与 `english-skill-prompts` 共享 `config.md` 与 `speclink/config.yaml` → **Mitigation**：明确宣告前置依赖 `add-omp-tool` 与 `english-skill-prompts`，待两者落地后再实施本变更，且 7 份 delta specs 均不与前两者修改同名 Requirement。
- **[本仓库切换 `locale: cn` 后对既有繁体 specs 的影响]** 本仓库既有正式规格以繁体中文撰写，新变更若用简体中文写 delta，修改既有 Requirement 时若不慎把名称转成简体会导致验证失败 → **Mitigation**：`instructions specs` 已明文规定指向正式规格的 Requirement 与 Scenario 名称须逐字沿用正式规格原文（不论原语言），且 `speclink validate` 会在封存前机械把关。

## Migration Plan

1. 既有显式设定 `tw`、`ja`、`en`、`auto` 的项目无须任何迁移、行为不变；原先依赖「未设定＝`English`」且仍需要英文的项目，显式执行 `speclink workflow-config set locale en`（及 `spec_locale en`）即可维持英文输出。
2. 需要简体中文的项目可执行 `speclink workflow-config set locale cn`（需要规格也用简体中文时加设 `speclink workflow-config set spec_locale auto` 或 `cn`，或保持未设定以沿用默认 `cn`），或在 Desktop 项目设定页下拉选取 `cn（简体中文）`。
3. 升级 CLI 后执行 `speclink update` 同步新版技能档。
4. 回滚方式：透过 git revert 并对本仓库改回 `locale: tw`、`spec_locale: tw` 即可无损还原。

## Open Questions

无。引擎未设定默认切换为 `cn`（显式 `en` 维持 `English`）、`cn`／简体优先顺序与范围（引擎＋设定页选项，不含整套桌面 UI 翻译包）已与使用者确认完毕。
