## Why

使用 Speclink 搭配 AI agent 進行規格驅動開發的開發者（團隊情境下包含 PO 與 PM），在初始化、提案、實作、審查、驗證、封存、提交與手冊生成等各階段都會讀取內嵌技能提示詞與工作區代理指令。雖然 `crates/engine/speclink-core/assets/skills/` 的內文主體已是英文且已配置共用對話語言段落（由模型依對話語言回覆），但其中 15 份技能原稿仍混雜 82 行繁體中文話術、選項標籤、交棒區塊與手冊契約字面量，加上本倉庫的 `speclink/config.yaml`（`context`、`rules`、`review`）與 `.claude/CLAUDE.md`、`AGENTS.md` 皆為繁體中文，導致非繁中語系使用者與多數英文為主的模型在遵循技能指令與解析固定輸出格式時適配性下降。在不改變原有語意、流程步驟與結構的前提下，將這些提示詞與指令統一為英文版本，並保持既有手冊頁向後相容。

## What Changes

- **內嵌技能原稿英文化（`crates/engine/speclink-core/assets/skills/`，共 15 份原稿）**：
  - 保留每一份技能的段落結構、步驟順序、判斷條件、CLI 指令、條件區塊與強調字紀律，只將殘留的繁體中文散文、引用訊息模板、選項文案、分隔句與範例改寫為語意等價的英文：
    - `apply-worktree-pre.md`：多變更等待提示、多 session 配方、未啟用 worktree 拒絕訊息、髒工作樹三選項標籤、建置成本提示與本體流程分隔句。
    - `apply-worktree-post.md`：六段 worktree 收尾與品質關卡交棒訊息。
    - `worktree-merge.md`：三段合併後交棒訊息與收尾指引句。
    - `archive.md`：候選清單狀態標籤（Ready to archive、Waiting on archive of、Prerequisite not yet archived、Broken metadata）與六段封存前後提示訊息。
    - `commit.md`：先封存子流程的六段封存前後提示訊息（與 `archive.md` 維持同義一致）。
    - `propose.md`：分期切分舉例、無路徑檔案描述舉例、`no-spec-change` 摘要標示與 `archiveAfter` 封存順序備註。
    - `audit.md`：三個稽核代理角色標題移除繁中括號譯名。
    - `discuss.md`：轉出狀態散文詞與進行中 delta 範例標題英文化（保留非英文主題轉英文 slug 的說明語意）。
    - `ingest.md`：轉出狀態散文詞與待重新反映徽章描述英文化。
    - `manual.md`：全文與手冊契約字面量英文化——來源行改為 `**Sources**:`、來源頁標題改為 `About this manual`、無矛盾標記改為 `None found`、無使用者面向能力標記改為 `No user-facing capabilities yet`、導覽模式觸發詞以 `tour` 為英文主詞、摘要區段改為 `## Manual Generation Summary`、remote 不支援與無手冊導覽提示全數改為英文，YAML 範例與錨點範例同步英文化。
    - 明確屬多語系 locale 對照說明的字面（如 `analyze.md`、`drift.md`、`verify.md`、`propose.md` 的 locale 顯示名舉例、`config.md` 的 locale 代碼對照與語言強度詞舉例、`review.md` 的多語系 finding 範例、`discuss.md` 的非英文主題轉 slug 範例）改為純英文說明或僅保留必要的多語系代碼對照，不殘留繁體中文散文或固定話術。
- **手冊來源行解析向後相容（`packages/ui/src/manualDoc.ts`）**：
  - 擴充來源行剝離正則，同時認得英文 `**Sources**:`、簡體中文 `**出处**：` 與既有繁體中文 `**出處**：`（半形與全形冒號皆可），確保既有手冊頁在介面渲染時不破版、無須回改舊檔。
- **技能原稿英文一致寫法守門與回歸測試（`speclink-core`）**：
  - 在 `crates/engine/speclink-core/tests/it/skill_authoring.rs` 新增機械檢查，防止繁體中文散文或未列入多語系範例白名單的漢字重新進入 `assets/skills/*.md`。
  - 同步更新 `crates/engine/speclink-core/tests/it/render_golden.rs` 中對技能英文話術的斷言。
  - 遞增 `crates/engine/speclink-core/src/workspace/init.rs` 的 `ASSET_VERSION`，同批更新 `crates/engine/speclink-core/tests/golden/assets.lock` 與全部工具的 golden 快照（`claude`、`claude-remote`、`claude-worktree`、`codex`、`copilot`、`pi`、`cursor`、`agy`、`omp`、`neutral-cli`、`neutral-tool-call`、`neutral-tool-call-remote`）。
  - 同步更新本倉庫受管技能目錄（`.claude/skills/`、`.agents/skills/`）。
- **本倉庫工作流配置與代理指令英文化**：
  - 將 `speclink/config.yaml` 中的 `context`、`rules`（`proposal`、`design`、`specs`、`tasks`）與 `review`（`critical`、`warning`）改寫為語意與條目結構完全等價的英文；保留 `schema`、`worktree`、`locale`、`spec_locale`、`tdd`、`audit` 的欄位名稱、預設值與現值不變（本倉庫規格與產出物語言設定不受影響）。
  - 將 `.claude/CLAUDE.md` 與 `AGENTS.md` 改寫為語意與段落結構完全等價的英文。

### 三種形態下的結果

- **本機 repo（filesystem 模式）**：可用。`speclink init` 與 `speclink update` 直接自 `speclink-core` 渲染全英文技能檔至各工具目錄；`packages/ui` 在 Desktop 讀取本機手冊時同時相容英文 `**Sources**:` 與既有中文來源行；`speclink instructions` 讀回本倉庫英文化的 `context` 與 `rules`。
- **連到遠端的 checkout（remote 模式）**：可用。遠端 Server 宣告技能時以相同 `speclink-core` 渲染函式回傳英文技能檔，未宣告或離線時退回本機範本渲染 `SpecLocation::Elsewhere` 英文版（含 `manual.md` 的英文 remote 限制提示）。
- **沒有本機規格檔的內嵌宿主（Node SDK / `createHost`）**：可用。`skills.render` 與 HTTP 技能端點經同一引擎渲染函式產出英文技能檔，不含任何本機規格路徑假設；不新增或變更任何 CLI 子指令、旗標、stdin 或 exit code。

### 相容性影響

- **人眼輸出與 `--json` 輸出**：CLI 所有既有子指令的人眼輸出、ANSI 行為、`--json` 欄位名稱與型別均不變更，無新增或異動設定欄位與預設值。
- **渲染技能檔與回歸對照（刻意變更）**：所有內建工具（`claude`、`codex`、`copilot`、`pi`、`cursor`、`agy`、`omp`）與自訂描述子的生成技能檔因原稿英文化與 `ASSET_VERSION` 遞增而變更，所有 `render_golden` 快照與 `assets.lock` 將於同批刻意更新。既有使用者升級引擎後執行一次 `speclink update` 即可無痛同步新版英文技能。
- **既有手冊頁相容性**：新生成的手冊改以 `**Sources**:`、`About this manual`、`None found` 為英文契約字面量；`packages/ui/src/manualDoc.ts` 維持向後相容，既有含 `**出處**：` 的手冊頁仍正常剝離來源行，既有使用者無須手動回改舊手冊。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `skill-authoring`: 新增技能原稿以英文撰寫的規範與機械守門，確保原稿散文、模板與選項標籤維持英文，僅於白名單的多語系說明例允許出現非英文示範詞。
- `archive-skill`: 更新候選清單狀態標籤與封存前後提示訊息的規格字面為英文版本，語意與流程分支不變。
- `commit-skill`: 更新先封存子流程的順序提示、重寫提醒與收尾提醒規格字面為英文版本，語意與流程分支不變。
- `propose-skill`: 更新 `no-spec-change` 收尾摘要顯示字面與 `archiveAfter` 封存順序備註為英文版本，語意與判斷規則不變。
- `worktree-apply-skill`: 更新未啟用 worktree 拒絕訊息、多變更等待／並行提示、髒工作樹三選項與本體流程分隔句為英文版本，語意與執行順序不變。
- `manual-pages`: 更新手冊頁尾來源行契約為 `**Sources**:`（讀取端向後相容既有 `**出處**：` 與 `**出处**：`），並將來源頁標題、無矛盾標記與無能力提示更新為英文契約字面。
- `manual-skill`: 更新手冊技能的導覽觸發詞、remote 限制提示、生成摘要模板與無手冊導覽提示為英文版本，生成與導覽流程語意不變。

## Impact

- 規格掃描結果：掃描 `speclink/specs/` 全部既有規格，辨識出 7 份直接相關的既有規格（`skill-authoring`、`archive-skill`、`commit-skill`、`propose-skill`、`worktree-apply-skill`、`manual-pages`、`manual-skill`）。其餘技能規格（如 `audit-skill`、`config-skill`、`discuss-skill`、`ingest-skill`、`worktree-merge-skill`）僅規範行為語意而未釘死繁中字面，無須新增或修改其規格，亦無須新增獨立 capability。
- Affected specs: skill-authoring, archive-skill, commit-skill, propose-skill, worktree-apply-skill, manual-pages, manual-skill
- 影響的技能與工具：影響 15 份內嵌技能原稿（`analyze`、`apply-worktree-pre`、`apply-worktree-post`、`archive`、`audit`、`commit`、`config`、`discuss`、`drift`、`ingest`、`manual`、`propose`、`review`、`verify`、`worktree-merge`），以及因 `ASSET_VERSION` 遞增而同步更新版號的全部渲染技能，涵蓋 `claude`、`codex`、`copilot`、`pi`、`cursor`、`agy`、`omp` 與自訂描述子。
- 影響的 crate 與 app：`speclink-core`、`packages/ui`，以及本倉庫工作流設定與代理指令檔（`speclink/config.yaml`、`.claude/CLAUDE.md`、`AGENTS.md`、`.claude/skills/`、`.agents/skills/`）。
- Affected code:
  - New: (none)
  - Modified:
    - crates/engine/speclink-core/assets/skills/analyze.md
    - crates/engine/speclink-core/assets/skills/apply-worktree-pre.md
    - crates/engine/speclink-core/assets/skills/apply-worktree-post.md
    - crates/engine/speclink-core/assets/skills/archive.md
    - crates/engine/speclink-core/assets/skills/audit.md
    - crates/engine/speclink-core/assets/skills/commit.md
    - crates/engine/speclink-core/assets/skills/config.md
    - crates/engine/speclink-core/assets/skills/discuss.md
    - crates/engine/speclink-core/assets/skills/drift.md
    - crates/engine/speclink-core/assets/skills/ingest.md
    - crates/engine/speclink-core/assets/skills/manual.md
    - crates/engine/speclink-core/assets/skills/propose.md
    - crates/engine/speclink-core/assets/skills/review.md
    - crates/engine/speclink-core/assets/skills/verify.md
    - crates/engine/speclink-core/assets/skills/worktree-merge.md
    - crates/engine/speclink-core/src/workspace/init.rs
    - crates/engine/speclink-core/tests/it/skill_authoring.rs
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
    - packages/ui/src/manualDoc.ts
    - packages/ui/src/__tests__/manualPage.test.tsx
    - speclink/config.yaml
    - .claude/CLAUDE.md
    - AGENTS.md
  - Removed: (none)
