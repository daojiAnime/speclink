## ADDED Requirements

### Requirement: 技能原稿以英文撰寫

crates/engine/speclink-core/assets/skills/ 下每一份技能原稿（含散文、提示引文、選項標籤、狀態標籤、分隔句、摘要範本與範例值）SHALL 全數以英文撰寫，SHALL NOT 含任何 Unicode 漢字（`\p{Han}`）；對使用者對話語言的適配由共用對話語言段落（`conversation-language.md`）與各步驟的 `locale` 綁定負責，而非於原稿內混寫非英文散文。此變更屬刻意更新渲染輸出：既有的英文技能語意、步驟順序、判斷條件、佔位符與強調字紀律維持不變，本機、遠端與內嵌宿主經單一渲染路徑取得相同英文原稿。

speclink-core 的整合測試（`cargo test -p speclink-core --test it skill_authoring::`）SHALL 逐份掃描 crates/engine/speclink-core/assets/skills/ 下全部 `.md` 原稿：任一原稿含任一 Unicode 漢字時測試 SHALL 以非零 exit code 失敗，失敗訊息 SHALL 指名原稿檔名、違反項目「英文撰寫」、命中的行號與片段，以及規範出處「skill-authoring 規格『技能原稿以英文撰寫』」。LF 與 CRLF 結尾的原稿 SHALL 得到相同結果。

#### Scenario: 全部技能原稿不含漢字時通過

- **WHEN** crates/engine/speclink-core/assets/skills/ 下全部 `.md` 原稿皆僅以英文撰寫，執行 `cargo test -p speclink-core --test it skill_authoring::`
- **THEN** 整合測試以 exit code 0 通過，不回報任何英文撰寫違規

#### Scenario: 技能原稿混入繁體中文時守門失敗

- **WHEN** 某份技能原稿含有一行繁體中文話術（例：`本專案未啟用 worktree 流程`），執行 `cargo test -p speclink-core --test it skill_authoring::`
- **THEN** 整合測試以非零 exit code 失敗，錯誤訊息指名該原稿檔名、「英文撰寫」、命中行號與該段漢字，以及規範出處

#### Scenario: LF 與 CRLF 原稿檢查結果一致

- **WHEN** 同一份含漢字的原稿分別以 LF 與 CRLF 行尾送入英文撰寫檢查
- **THEN** 兩者皆回報相同行號與相同的違規項目
