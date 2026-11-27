## MODIFIED Requirements

### Requirement: Ambiguity 的弱語氣詞偵測——英文樣式 should、may、might、consider、possibly 以字邊界比對

<!-- BEFORE: CJK 弱語氣詞清單僅含繁體詞形（應該、也許、或許、大概、考慮、盡量、儘量、待定、可能），不含簡體字形。 -->

每一非標題行 SHALL 至多報一筆 `ambWeakLanguage`（Suggestion），`location` 為 `<delta 相對路徑>:<行號>`，`summary` 為 `Vague language '<pattern>' found`。檢查順序 SHALL 為：五個英文樣式（依上述順序）、再 `TBD`／`TODO`／`???`／`TKTK`、再 CJK 樣式。五個英文樣式 SHALL 在整行轉小寫後以字邊界比對：命中子字串的前後字元都不是 ASCII 字母才算，`n't` 縮寫視為字邊界（`shouldn't` 仍報 `should`）；代價是 `maybe`、`considered` 這類含樣式的完整單字不再命中。`TBD`／`TODO`／`TKTK` SHALL 維持大小寫不敏感的子字串比對，`???` 維持原樣子字串比對。CJK 樣式 SHALL 同時涵蓋繁體字形（`應該`、`也許`、`或許`、`大概`、`考慮`、`盡量`、`儘量`、`待定`）與簡體字形（`应该`、`也许`、`或许`、`考虑`、`尽量`）並維持子字串比對，「不可能」豁免照舊（該三字去除後仍含「可能」才報）。以 `#` 開頭的標題行 SHALL NOT 掃描。此為刻意變更：新增簡體中文弱語氣詞偵測，既有英文與繁體中文比對行為維持不變。

#### Scenario: 字邊界擋住 shoulder

- **WHEN** spec 行為 `The shoulder strap SHALL lock`
- **THEN** 不報 `ambWeakLanguage`

#### Scenario: 獨立的 should 仍命中

- **WHEN** spec 行為 `The strap should lock`
- **THEN** 報一筆 Suggestion，`summary` 為 `Vague language 'should' found`

#### Scenario: CJK 樣式維持子字串——可能、應該、考慮

- **WHEN** spec 行為 `系統盡可能鎖定`
- **THEN** 報一筆 Suggestion，`summary` 為 `Vague language '可能' found`；行為 `這不可能發生` 時不報

#### Scenario: 簡體中文弱語氣詞命中

- **WHEN** spec 行為 `系统应该在超时后尽量重试`
- **THEN** 報一筆 Suggestion，`summary` 為 `Vague language '应该' found`

##### Example: 字邊界與簡繁 CJK 判定

| spec 行 | 結果 |
| --- | --- |
| `it should lock` | `should` |
| `Should lock` | `should`（先轉小寫） |
| `the shoulder strap` | 不報 |
| `a considerable delay` | 不報 |
| `the mayor` | 不報 |
| `outbdoor` | `TBD`（子字串比對維持） |
| `maybe lock` | 不報（字邊界的代價） |
| `系统应该锁住` | `应该` |
| `尽量减少等待` | `尽量` |
| `考虑缓存结果` | `考虑` |
| `这不可能发生` | 不報（不可能豁免） |
