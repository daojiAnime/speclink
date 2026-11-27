## Context

Speclink 的內嵌技能原稿位於 `crates/engine/speclink-core/assets/skills/`，透過 `speclink-core` 單一渲染路徑服務所有內建工具（`claude`、`codex`、`copilot`、`pi`、`cursor`、`agy`、`omp`）、自訂描述子、遠端 Server 技能端點與 Node SDK。雖然原稿主體已採英文並於渲染時自動插入 `conversation-language.md`（指示模型依使用者對話語言翻譯範例散文並保留固定 token），但其中 15 份原稿仍殘留 82 行繁體中文話術、選項文案、手冊契約字面與摘要模板；同時本倉庫的 `speclink/config.yaml`（`context`、`rules`、`review`）與 `.claude/CLAUDE.md`、`AGENTS.md` 皆以繁體中文撰寫。

這造成兩類實際問題：
1. 非繁中語系開發者與偏重英文指令遵循的模型在執行 `archive`、`commit`、`propose`、`apply-with-worktree`、`worktree-merge`、`manual` 等技能時，會讀到混雜繁體中文的固定引文、選項標籤與分隔句，降低跨語言與跨模型適配性。
2. `manual.md` 中的手冊頁尾來源行（`**出處**：`）與 `packages/ui/src/manualDoc.ts` 的 `SOURCES_LINE_RE` 緊耦合，改動時必須保證既有繁體中文手冊頁不破版。

## Goals / Non-Goals

**Goals:**
- 將 `crates/engine/speclink-core/assets/skills/*.md` 全部 15 份含繁體中文的原稿改為自然、精確的英文，嚴格維持原有步驟編號、條件區塊（`{{#SPEC_IN_CHECKOUT}}`／`{{^SPEC_IN_CHECKOUT}}`）、渲染佔位符（`{{SPEC_DIR}}`、`{{TOOL}}`、`{{PLAN_DIR}}`、`{{NEXT_STEPS_LEAD}}`、`{{ARTIFACT_WRITE_BACK}}`、`/speclink:<name>`）、強調字上限與語意邏輯不變。
- 將 `manual.md` 與手冊契約字面量統一為英文（`**Sources**:`、`About this manual`、`None found`、`No user-facing capabilities yet`、`## Manual Generation Summary`、`tour` 觸發詞等），同時讓 `packages/ui/src/manualDoc.ts` 向後相容既有 `**出處**：` 與簡體 `**出处**：` 來源行。
- 在 `crates/engine/speclink-core/tests/it/skill_authoring.rs` 新增機械守門，防止未經白名單許可的漢字散文再次混入技能原稿。
- 遞增 `ASSET_VERSION`、更新 `assets.lock` 與全套 golden 快照及 `render_golden.rs` 斷言，並以新版引擎更新本倉庫 `.claude/skills/` 與 `.agents/skills/`。
- 將本倉庫的 `speclink/config.yaml`（`context`、`rules`、`review`）與 `.claude/CLAUDE.md`、`AGENTS.md` 改為語意與結構等價的英文版本。

**Non-Goals:**
- 不改變 `speclink/config.yaml` 的 `schema`、`worktree`、`locale`、`spec_locale`、`tdd`、`audit` 設定值與設定檔 schema（本倉庫仍維持 `locale: tw`、`spec_locale: tw`）。
- 不修改 `crates/engine/speclink-core/assets/schema/*/fork.schema.yaml` 中專為多語系任務格式示範而設的 Traditional Chinese／Japanese 範例行，也不修改 `crates/engine/speclink-core/src/workspace/instructions.rs` 對 `tw` 規格語言注入的中文模糊用語提醒。
- 不修改 CLI 子指令、旗標、`--json` payload 形狀、Rust 程式碼註解、前端 UI i18n 文案（`packages/ui/src/i18n.tsx`、`apps/desktop/src/i18n/messages.ts`）、Herdr i18n、既有 `docs/*.zh-TW.md` 文件、既有 `speclink/specs/` 既有正文或 `integrations/claude-code/skill-evals/` 評測題目。
- 不重寫或增刪任何技能的工作流程步驟、判斷條件或路由關係。

## Decisions

### D1：內嵌技能原稿英文化與結構／語意等價原則

**選擇**：以 `crates/engine/speclink-core/assets/skills/*.md` 為唯一事實來源（single source of truth），逐檔翻譯全部 15 份原稿的繁體中文行，使所有本機 repo、遠端 checkout 與內嵌宿主（CLI／Server／Node SDK／Desktop）共用同一份英文技能原稿，不另設平行翻譯層或依工具分叉。

**具體翻譯邊界**：
- **固定話術、選項與分隔句（一律改為英文）**：
  - `apply-worktree-pre.md`：等待前置提示（`<change-name> must wait until <prerequisites> lands before starting.`）、多 session 配方（`The parallel workflow is one change per session: open separate windows and run /speclink:apply-with-worktree <change-name> in each. The main checkout board displays progress for every worktree at once.`）、未啟用拒絕訊息（`This project has not enabled the worktree workflow. To enable it, run: speclink workflow-config set worktree true`）、髒工作樹三選項（`Commit code first, then open worktree`、`Continue anyway`、`Stop`）、建置成本提示（`A worktree is a full copy of the source code. Dependencies and build artifacts are not copied over — the first test or build run inside it will need to install dependencies and rebuild, which can take a while.`）、本體分隔句（`The apply workflow follows below; execute it inside the worktree directory above.`）。
  - `apply-worktree-post.md`：六段交棒引文全數譯為英文，保留原分支條件（本機 vs 遠端、有無未勾手動任務、CI 看不到本機 worktree 分支的說明與 `/speclink:review` / `/speclink:verify` / `/speclink:quality` / `/speclink:worktree-merge` 記號）。
  - `worktree-merge.md`：三段合併後交棒引文與 `Next:` 總結行譯為英文，保留三種狀態分岔與正典順序。
  - `archive.md` 與 `commit.md`：候選狀態標籤改為 `Ready to archive`、`Waiting on archive of <archiveAfter names>`、`Prerequisite <blockedBy names> not yet archived`、`Broken metadata`；六段封存前後提示引文（`archiveAfter` 建議先封存並重寫重疊 requirement、`conflict` 提醒改其中一邊、`blockedBy` 提醒、封存後提醒排在後面的變更重寫、下一個可開工 `Next ready to start from plan: <next> — run /speclink:apply <next>.`、第一波可並行 `Wave 1 parallel-safe: <name-a>, <name-b> — open one session per change and run /speclink:apply-with-worktree <name>.`）在兩檔間維持逐字一致的英文表述。
  - `propose.md`：分期舉例改為 `cut 1 / cut 2 / ..., cut A / cut B`、無路徑檔案描述舉例改為 `"the parser entry file"`、`no-spec-change` 顯示字面改為 `no-spec-change (no spec changes)`、`archiveAfter` 備註改為 `When archiving, <change> must come after <archiveAfter names>.`。
  - `audit.md`：三個代理標題改為 `**Agent 1 — The Scoundrel**`、`**Agent 2 — The Lazy Developer**`、`**Agent 3 — The Confused Developer**`。
  - `discuss.md` 與 `ingest.md`：將散文中的 `已轉出` 改為 `promoted`、`待重新反映` 改為 `needs re-ingest`、`討論資訊 payload` 範例改為 `Discussion metadata payload`。
- **多語系說明例的處理**：
  - `analyze.md`、`drift.md`、`verify.md` 的 `locale` 舉例改為純英文範例（如 `"English"` 或 `"Traditional Chinese"`，不帶括號漢字）；`propose.md` 的 `locale` 舉例改為 `"Japanese"`；`review.md` 改為以英文範例示範 finding 格式並說明非英文 locale 依 `locale` 翻譯其餘敘述；`discuss.md` 的非英文主題轉 slug 範例改以拉丁語系／說明文字示範（如 `e.g., translating a non-English title into an English kebab-case slug like board-search-bar`）；`config.md` 的 locale 代碼對照改以英文名稱對照（`"Traditional Chinese" -> tw`, `"Japanese" -> ja`, `"English" -> en`）且強度提示詞保留英文清單（`"never", "always", "must", "prefer"`，並註明其他語言的對應強度詞同等看待）。
  - 這樣一來，全部 28 份技能原稿可達成零漢字殘留，讓機械守門簡單且無須維護脆弱的行級豁免清單。

**替代方案與排除理由**：
- 方案 B（依 `locale` 維護多套語言原稿 `assets/skills/{en,tw}/`）：被否決——`conversation-language.md` 已負責讓模型以使用者對話語言回覆，維護多語原稿會導致重複維護與語意漂移，違反 KISS 原則。
- 方案 C（在守門內維護每份原稿的漢字行白名單）：被否決——白名單容易隨改動腐爛，上述 7 行多語系舉例用英文表述完全不損語意，零白名單更乾淨。

### D2：手冊契約字面量英文化與 manualDoc 向後相容解析

**選擇**：
- 在 `crates/engine/speclink-core/assets/skills/manual.md` 中將手冊契約字面量與摘要模板統一改為英文：
  - 導覽模式觸發詞以 `tour` 為主（表格條件 `contain tour`），remote 限制訊息改為 `Manual generation is not yet supported in remote mode (tour mode still works: /speclink:manual tour)`。
  - 頁尾來源行改為 `**Sources**:`（例：`**Sources**: \`github-oauth\`, \`user-pending-blocked-pages\``）。
  - `about.md` 標題改為 `About this manual`；無矛盾標記改為 `None found`；無使用者面向能力標記改為 `No user-facing capabilities yet`。
  - 生成摘要模板改為 `## Manual Generation Summary`，各列改為英文（Added / Regenerated / Stamp-only / Untouched / Possibly stale / Unlisted capabilities / Contradictions recorded on about page / Skipped (unparseable frontmatter) or missing sources），結尾建議句改為 `Consider committing the manual changes with a regular git commit.`。
  - 無手冊導覽提示改為 `No manual yet — touring directly from the specs`。
  - YAML frontmatter 與錨點範例改用英文示範值（如 `title: First Login`、`section: Getting Started`、`sources: ["desktop-app#Board and Tasks", "desktop-app#Tray Menu", policy-config]`）。
- 在 `packages/ui/src/manualDoc.ts`（共用層唯一實作落點，Desktop 與任何手冊頁皆透過 `@speclink/ui` 的 `ManualPage` 呼叫 `stripSourcesLine`，不在 `apps/*` 另寫解析）將 `SOURCES_LINE_RE` 擴充為 `/^\*\*(?:Sources|出处|出處)\*\*[：:]/`：
  - 新生成的英文 `**Sources**:`、簡體 `**出处**：` 與既有的繁體 `**出處**：`（半形 `:` 與全形 `：` 皆相容）都能正常剝離，不破壞既有工作區手冊。

**替代方案與排除理由**：
- 只匹配 `**Sources**:` 而移除 `**出處**：`：被否決——會破壞既有使用者工作區已生成的手冊頁顯示，違反向後相容原則。

### D3：技能原稿英文一致寫法的機械守門（skill_authoring）

**選擇**：在 `crates/engine/speclink-core/tests/it/skill_authoring.rs` 新增第七項檢查（對應 `skill-authoring` 新增需求 `技能原稿以英文撰寫`），掃描 `crates/engine/speclink-core/assets/skills/` 底下全部 `.md` 原稿，檢查全文不含任何 Unicode Han 字元（`\p{Han}`）。違規時回報原稿檔名、檢查項名、命中的行號與片段，以及規範出處。

**原因**：
- 零白名單設計讓守門完全確定且可獨立單元測試（含正例與反例測試），防止未來更新技能原稿時再次混入繁體中文散文或話術。
- 此規則歸入 `skill-authoring` 的新增 Requirement，不修改既有的 `技能原稿的跨模型寫法規範` 或 `寫法規範的機械守門` 條文，避免與進行中的 `add-omp-tool` 變更產生 requirement 層級重疊。

### D4：產物層版號遞增、golden 與受管技能同步

**選擇**：
- 遞增 `crates/engine/speclink-core/src/workspace/init.rs` 的 `ASSET_VERSION`（若實施時 `add-omp-tool` 已先落地為 `v1.64.0`，則遞增至 `v1.65.0`；依實施當下 `init.rs` 的現值加一個 minor 版本）。
- 更新 `crates/engine/speclink-core/tests/it/render_golden.rs` 中所有對舊繁中字面的斷言，改為對等價英文片語的斷言，同時保留反向斷言確保不殘留舊誤導用語。
- 以 `UPDATE_GOLDEN=1 cargo test -p speclink-core --test it render_golden::` 同步重寫 `crates/engine/speclink-core/tests/golden/assets.lock` 與全部 12 份技能快照（含 `omp.snapshot.md`）。
- 建置最新 CLI 執行 `speclink update` 同步本倉庫的 `.claude/skills/` 與 `.agents/skills/`。
- 所有技能原稿維持在既有 `EMPHASIS_CAPS` 上限內且不超過 500 行，若翻譯後某檔強調字減少不須下調上限，絕不無故放寬上限。

### D5：本倉庫工作流設定（config.yaml）與代理指令（CLAUDE.md／AGENTS.md）英文化

**選擇**：
- 翻譯 `speclink/config.yaml` 的 `context`、`rules`（`proposal`、`design`、`specs`、`tasks`）與 `review`（`critical`、`warning`）為結構與條目數完全一致的英文；保留 YAML 頂層所有鍵與 `schema`、`worktree`、`locale: tw`、`spec_locale: tw`、`tdd: true`、`audit: true` 不動。
  - 注意：`speclink/config.yaml` 是手動一次重寫本倉庫設定檔，而非透過引擎寫入動詞；重寫後須以 `speclink workflow-config show --json` 驗證 YAML 可完整解析且鍵值結構與原本一致（序列化向後相容不變）。
- 翻譯 `.claude/CLAUDE.md` 與 `AGENTS.md` 為結構與規則完全等價的英文（保留 `conventional-commit` 規則中說明 commit description/body 語系的原意、工具查詢優先序、程式碼取捨原則與回覆習慣）。

## Implementation Contract

- **Behavior**：
  - 呼叫 `speclink init`、`speclink update`、`speclink instructions --skill <name>`、Node SDK `skills.render` 或遠端技能端點取得的全部技能檔，其內文（含話術引文、選項標籤、狀態標籤、分隔句、手冊契約字面量與摘要模板）均為英文，不含任何繁體中文或漢字，且原有步驟編號、條件分岔、佔位符與 CLI 動詞指令語意與結構完全維持不變。
  - `packages/ui` 的 `stripSourcesLine` 在手冊頁渲染時，能剝離頁尾以 `**Sources**:`、`**出处**：` 或 `**出處**：`（支援半形 `:` 與全形 `：`）開頭的最後一個非空行；不符模式時原樣回傳。
  - 本倉庫執行 `speclink workflow-config show --json` 與 `speclink instructions <artifact> --change <name> --json` 時，回傳的 `context`、`rules` 與 `review` 為英文，且 `schema`、`worktree`、`locale`、`specLocale`、`tdd`、`audit` 值與變更前一致。
- **Interface / Data Shape**：
  - `crates/engine/speclink-core/src/workspace/init.rs`：`ASSET_VERSION` 遞增一個 minor 版號。
  - `crates/engine/speclink-core/assets/skills/*.md`：15 份原稿英文化，保留全部既定標記（`{{#SPEC_IN_CHECKOUT}}`、`{{^SPEC_IN_CHECKOUT}}`、`{{/SPEC_IN_CHECKOUT}}`、`{{SPEC_DIR}}`、`{{TOOL}}`、`{{PLAN_DIR}}`、`{{NEXT_STEPS_LEAD}}`、`{{ARTIFACT_WRITE_BACK}}`、`/speclink:<name>`）。
  - `packages/ui/src/manualDoc.ts`：`stripSourcesLine(body: string): string` 簽名不變，內部 `SOURCES_LINE_RE` 擴充為 `/^\*\*(?:Sources|出处|出處)\*\*[：:]/`。
  - `speclink/config.yaml`：YAML 鍵結構不變（`schema`、`worktree`、`locale`、`spec_locale`、`tdd`、`audit`、`context`、`rules.{proposal,design,specs,tasks}`、`review.{critical,warning}`）。
  - CLI 子指令、旗標、`--json` 欄位與 `speclink-core` 公開 API 簽名皆無變更。
- **Failure Modes**：
  - 任一 `assets/skills/*.md` 出現漢字時，`cargo test -p speclink-core --test it skill_authoring::` 以非零 exit code 失敗並指出原稿檔名、行號與違規文字。
  - 若改動技能原稿卻漏升 `ASSET_VERSION` 或漏更新 `assets.lock`／golden 快照，`cargo test -p speclink-core --test it render_golden::` 失敗並指示修正方式。
  - 若 `speclink/config.yaml` 翻譯時出現 YAML 語法錯誤，`speclink workflow-config show --json` 依既定 fail-closed 行為回報解析錯誤並以非零 exit code 結束。
  - 手冊頁尾無來源行或為空字串時，`stripSourcesLine` 靜默回傳原字串不拋錯。
- **Acceptance Criteria**：
  - `cargo test -p speclink-core --test it skill_authoring::`、`skill_verbization::` 與 `render_golden::` 全數通過。
  - `cargo test -p speclink-core` 與 `cargo test -p speclink-cli` 通過。
  - `npm test -w @speclink/ui` 通過（含英文 `**Sources**:`、簡體 `**出处**：` 與既有繁體 `**出處**：` 的 `stripSourcesLine` 單元測試與 `ManualPage` 渲染測試）。
  - `speclink workflow-config show --json` 回傳 exit code 0 且 `locale === "tw"`、`specLocale === "tw"`、`context`/`rules`/`review` 為完整英文。
  - `node --test scripts/*.test.mjs scripts/*/*.test.mjs` 全數通過。
- **Scope Boundaries**：
  - **In scope**：15 份內嵌技能原稿、`ASSET_VERSION`、golden 快照與 `assets.lock`、`skill_authoring.rs` 與 `render_golden.rs` 測試、`packages/ui/src/manualDoc.ts` 與測試、`speclink/config.yaml` 提示內容、`.claude/CLAUDE.md`、`AGENTS.md`、本倉庫 `.claude/skills/` 與 `.agents/skills/` 同步。
  - **Out of scope**：`locale`/`spec_locale` 預設或本倉庫值、`assets/schema/*/fork.schema.yaml`、`instructions.rs` CJK 弱語言提醒、前端 UI i18n、Herdr i18n、Rust 註解、`docs/*.zh-TW.md`、`integrations/claude-code/skill-evals/`。

## Risks / Trade-offs

- **[回歸對照與測試斷言遺漏]** 改動 15 份技能原稿會影響所有內建與中性工具的 golden 快照、`assets.lock` 以及 `render_golden.rs` 中大量中文子字串斷言 → **Mitigation**：先以 `rg '\p{Han}' crates/engine/speclink-core/tests/it/render_golden.rs` 盤點每一條中文斷言並逐一對應改寫，再跑 `UPDATE_GOLDEN=1` 與完整 `cargo test -p speclink-core`、`cargo test -p speclink-cli` 確認零意外飄移。
- **[與進行中變更 `add-omp-tool` 共享 `init.rs`／golden／`assets.lock`]** `add-omp-tool` 同樣動到 `ASSET_VERSION`、golden 與 `skill_authoring.rs` → **Mitigation**：以 `speclink change depends` 宣告本變更依賴 `add-omp-tool`，待 `add-omp-tool` 落地後再套用本變更並依最新 `ASSET_VERSION` 遞增，且本變更在 `skill-authoring` 使用 `ADDED Requirements` 不與其 `MODIFIED` 條目衝突。
- **[跨平台行尾與路徑行為（Windows / macOS / Linux）]** `skill_authoring.rs` 與 `render_golden.rs` 同時受 LF 與 CRLF 影響 → **Mitigation**：新增的英文一致寫法守門逐行以 Unicode `\p{Han}` 比對（對 LF／CRLF 無感），並比照既有 `line_count_is_the_same_for_lf_and_crlf` 補跨行尾測試。
- **[翻譯時無意增加大寫強調字或改變語意／結構]** 英文習慣常自然寫出 `MUST`/`NOT`/`NEVER` 導致觸發 `EMPHASIS_CAPS`，或遺漏既有條件分支 → **Mitigation**：每個原稿翻譯後逐段對照原結構、使用小寫 `must`/`not`/`never`（除原稿既有大寫強調處外），並以 `skill_authoring::` 守門把關。
- **[既有工作區舊版手冊頁相容性]** 既有使用者已產出含 `**出處**：` 的手冊頁 → **Mitigation**：`packages/ui/src/manualDoc.ts` 的 `SOURCES_LINE_RE` 同時支援 `Sources`、`出处`、`出處` 三種前綴與半形／全形冒號，不破壞舊頁。

## Migration Plan

1. 既有使用者升級 Speclink 後執行 `speclink update`，因 `ASSET_VERSION` 遞增，CLI 會自動以最新英文技能覆寫受管的 `speclink-*` 技能檔。
2. 既有工作區的 `speclink/manual/` 手冊頁無須手動遷移；UI 讀取端同時相容舊 `**出處**：` 與新 `**Sources**:`，下次執行 `/speclink-manual` 重生過期頁時會平滑轉成英文契約字面。
3. 回滾方式：若發現任何語意偏差，透過 git revert 本次變更並重跑 `speclink update` 即可完整復原。

## Open Questions

無。範圍與手冊契約字面量策略已與使用者確認完畢。
