//! Project setup verbs: init and update.
//!
//! ModeFree: both write the local workspace scaffolding. `init --store remote`
//! still writes locally — it records the connection rather than consuming it.

use anyhow::{bail, Result};
use clap::Args;
use speclink_core as core;
use std::io::IsTerminal;
use std::path::PathBuf;

use crate::color;
use crate::remote_base::{connect, validate_or_defer};
use core::init::{ToolSelection, ASSET_VERSION};
use speclink_protocol::skills::SkillsRequest;
use speclink_remote::skills as remote_skills;

#[path = "init_tui.rs"]
mod init_tui;

#[derive(Args)]
pub(crate) struct InitArgs {
    /// Project path (defaults to current directory)
    path: Option<String>,
    /// AI tools to generate files for (e.g., claude, codex, copilot, pi, cursor, agy, omp)
    #[arg(long)]
    tools: Option<String>,
    /// Overwrite existing files
    #[arg(long)]
    force: bool,
    /// Custom openspec directory path (default: openspec)
    #[arg(long)]
    dir: Option<String>,
    /// Store backend: fs (default) or remote
    #[arg(long, default_value = "fs")]
    store: String,
    /// Remote store connection URL (required with --store remote)
    #[arg(long)]
    url: Option<String>,
    /// This repo's registered name in the remote project
    #[arg(long)]
    repo: Option<String>,
}
#[derive(Args)]
pub(crate) struct UpdateArgs {
    /// Project path (defaults to current directory)
    path: Option<PathBuf>,
    /// Overwrite existing files
    #[arg(long)]
    force: bool,
    /// Rewrite managed skill files even when the workspace is newer than this engine
    #[arg(long)]
    allow_downgrade: bool,
}
pub(crate) fn cmd_init(a: InitArgs) -> Result<()> {
    // The success line echoes the PATH argument verbatim (`init .` prints ".\openspec");
    // the absolute path is only used internally.
    let display_base = match a.path.as_deref() {
        Some(p) => p.to_string(),
        None => std::env::current_dir()?.display().to_string(),
    };
    let root = match a.path.as_deref() {
        Some(p) => {
            let pb = PathBuf::from(p);
            if pb.is_absolute() { pb } else { std::env::current_dir()?.join(pb) }
        }
        None => std::env::current_dir()?,
    };
    match a.store.as_str() {
        "fs" | "remote" => {}
        other => bail!("Unknown store '{other}'. Use 'fs' or 'remote'."),
    }
    // Tools are resolved BEFORE the fs/remote split, so both paths start from the same
    // validated non-empty selection and no store writes anything until it is settled.
    let interactive = std::io::stdin().is_terminal();
    let tools = resolve_init_tools(a.tools.as_deref(), interactive, || {
        init_tui::run_tui_tool_picker(
            &root,
            &display_base,
            &a.store,
            a.url.as_deref(),
            color::paint(),
            &mut std::io::stderr(),
        )
    })?;
    if a.store == "remote" {
        return cmd_init_remote(&a, &root, &display_base, &tools);
    }
    let spec_dir = a.dir.clone().unwrap_or_else(|| core::workspace::DEFAULT_SPEC_DIR.to_string());
    let outcome = core::init::init(&root, &tools, a.force, &spec_dir)?;
    if let Some(report) = &outcome.imported {
        println!("{} Imported the OpenSpec project: {}", color::green("✓"), moved_summary(report));
    }
    // A legacy project's --force re-init stays in openspec/: name the folder init used.
    let folder = outcome.spec_dir_abs.strip_prefix(&root).unwrap_or(&outcome.spec_dir_abs);
    println!(
        "{} Initialized at {display_base}{}{}",
        color::green("✓"),
        std::path::MAIN_SEPARATOR,
        folder.display()
    );
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    println!("Generated files for: {}", names.join(", "));
    if outcome.imported.is_some() {
        println!("! Commit the import: speclink/ (added), openspec/ (deleted), .speclink.yaml, .gitignore and the generated skill files");
        println!("! The OpenSpec CLI no longer sees this project — the move is one-way");
        println!("! OpenSpec's own skills and command files were not moved or deleted — remove them yourself if you no longer use OpenSpec");
    }
    Ok(())
}
/// The single line every missing/empty selection ends on — it names the flag and every
/// valid tool, so a failed non-interactive run is self-correcting.
const TOOLS_HINT: &str = "no AI tool selected — pass --tools with one or more of claude, \
codex, copilot, pi, cursor, agy, omp (comma-separated, e.g. --tools claude,omp)";
/// Resolve init's built-in tool selection. An explicit `--tools` is validated and used
/// as-is (no prompt). Without the flag an interactive terminal opens the `npx skills`-style
/// TUI multi-select picker on stderr; a non-interactive terminal fails before any core write.
fn resolve_init_tools(
    spec: Option<&str>,
    interactive: bool,
    run_picker: impl FnOnce() -> Result<Vec<core::skills::Tool>>,
) -> Result<Vec<core::skills::Tool>> {
    match spec {
        Some(spec) => {
            let tools = core::init::parse_tools(spec)?;
            if tools.is_empty() {
                bail!("{TOOLS_HINT}");
            }
            Ok(tools)
        }
        None if interactive => run_picker(),
        None => bail!("{TOOLS_HINT}"),
    }
}
pub(crate) fn cmd_update(a: UpdateArgs) -> Result<()> {
    let root = a
        .path
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let root = if root.is_absolute() {
        root
    } else {
        std::env::current_dir()?.join(root)
    };
    if !core::workspace::is_project_root(&root) {
        bail!("Not initialized. Run 'speclink init' to initialize.");
    }
    let _ = a.force;
    // 降級守門在引擎的 update 本體（唯一實作落點）；--allow-downgrade 是唯一越過。
    // remote 專案的技能跟著 Server：Server 有給就不經這道守門。
    let local = || core::init::update(&root, a.allow_downgrade);
    let (outcome, source) = match skills_from_server(&root)? {
        FromServer::NotRemote => (local()?, None),
        FromServer::NotGiven(reason) => {
            eprintln!("speclink: skills generated from this CLI's templates — {reason}");
            (local()?, Some(format!("local ({ASSET_VERSION})")))
        }
        FromServer::Installed(remote_skills::Installed { outcome, version, replaced }) => {
            if let Some(had) = replaced {
                println!("! Skills replaced with the server's version {version} (this checkout had {had})");
            }
            (outcome, Some(format!("server ({version})")))
        }
    };
    print_migration(&outcome.migration, &outcome.migrated);
    for note in &outcome.notes {
        println!("! {note}");
    }
    // 棄用提示走 stderr：不是錯誤、不影響 exit code，也不該混進 stdout 的機器面。
    for note in &outcome.deprecations {
        eprintln!("! {note}");
    }
    let nothing_done = outcome.updated.is_empty() && outcome.pruned.is_empty() && outcome.stripped.is_empty();
    if nothing_done && outcome.unserved.is_empty() {
        println!("! No AI tool configurations found. Use 'speclink init --tools' to set up.");
    } else {
        if !outcome.updated.is_empty() {
            println!("{} Updated skill files for: {}", color::green("✓"), outcome.updated.join(", "));
        }
        if !outcome.pruned.is_empty() {
            println!(
                "! Pruned generated files for deselected tool: {}",
                outcome.pruned.join(", ")
            );
        }
    }
    // Server 一支技能都不給的工具：沒有寫入也沒有清理，要講明是哪些。
    if !outcome.unserved.is_empty() {
        println!(
            "! The server gives no skills for: {} — their skill files were left as they are",
            outcome.unserved.join(", ")
        );
    }
    if let Some(source) = source {
        println!("  Skills source: {source}");
    }
    // 遺留剝除的摘要（design D2）：舊版引擎注入的區塊被移除時明說改了哪些檔案，
    // 使用者才知道 diff 裡多出的變動從何而來。
    if !outcome.stripped.is_empty() {
        println!(
            "! Stripped legacy Speclink blocks from: {}",
            outcome.stripped.join(", ")
        );
    }
    Ok(())
}
/// What asking the Server for a project's skills came to.
enum FromServer {
    /// A local project, or a remote one with no tool to ask for: no Server is asked.
    NotRemote,
    /// The Server gives no skills this run, for this reason (one line): it declares
    /// none, or the handshake did not succeed. The local templates stand in.
    NotGiven(String),
    /// The Server's skills are in the checkout.
    Installed(remote_skills::Installed),
}

/// Take a remote project's skills from its Server (spec workspace-tools「remote 專案的
/// 技能取自 Server」). Once the handshake declares skills there is no way back to the
/// local templates: a request that fails then is an error, and nothing is written —
/// the Server said it has the skills, and other text in their place could tell the
/// agent to use verbs this Server does not have.
fn skills_from_server(root: &std::path::Path) -> Result<FromServer> {
    let app = core::config::AppConfig::load(&root.join(".speclink.yaml"))?;
    if app.remote.is_none() {
        return Ok(FromServer::NotRemote);
    }
    let selection = ToolSelection::resolve(root, &app);
    // 壞描述子＝零寫入、零請求：和本機的同步同一條規則。
    if let Some(message) = &selection.descriptor_error {
        bail!("{message}");
    }
    let targets = remote_skills::targets_of(&selection);
    if targets.is_empty() {
        return Ok(FromServer::NotRemote);
    }
    let Some(ws) = core::workspace::Workspace::discover(root)? else {
        return Ok(FromServer::NotRemote);
    };
    let conn = match speclink_host::context::resolve_store_mode(&ws) {
        Ok(core::workspace::ModeResolution { mode: core::workspace::StoreMode::Remote(conn), .. }) => conn,
        Ok(_) => return Ok(FromServer::NotRemote),
        Err(e) => return Ok(FromServer::NotGiven(e.to_string())),
    };
    let connected = match connect(&conn) {
        Ok(connected) => connected,
        Err(line) => return Ok(FromServer::NotGiven(line)),
    };
    if connected.binding.capabilities.skills.is_none() {
        return Ok(FromServer::NotGiven("this server does not provide skills".to_string()));
    }

    let refused = |e: &dyn std::fmt::Display| anyhow::anyhow!("could not get the skills from the server: {e}");
    let request = SkillsRequest { targets, contents: true };
    let response = connected.client.skills(&request).map_err(|e| refused(&e))?;
    let installed = remote_skills::install(root, selection, response).map_err(|e| refused(&e))?;
    Ok(FromServer::Installed(installed))
}

/// The spec directory migration's lines (spec-dir-migration D5): what moved and
/// what to do next, or why it waits.
fn print_migration(plan: &core::migrate::MigrationPlan, report: &core::migrate::MigrationReport) {
    use core::migrate::MigrationPlan;
    match plan {
        MigrationPlan::Nothing => {}
        MigrationPlan::MoveFolder => {
            println!("{} Migrated the spec folder: {}", color::green("✓"), moved_summary(report));
            println!("! Commit the move: speclink/ (added) and openspec/ (deleted)");
            println!("! Teammates must upgrade speclink before pulling that commit — older versions see an empty project");
            println!("! Update your own scripts, CI settings and .gitignore entries that still name openspec/");
        }
        MigrationPlan::RenameLeftovers => {
            println!(
                "{} Renamed {} change data file(s) in speclink/ from .openspec.yaml to .meta.yaml — commit them",
                color::green("✓"),
                report.renamed
            );
        }
        MigrationPlan::Deferred(reason) => {
            println!("! Spec folder migration skipped: {reason}");
        }
    }
}

/// The folder move's result, shared by `update`'s migration and `init`'s import.
fn moved_summary(report: &core::migrate::MigrationReport) -> String {
    format!(
        "moved openspec/ to speclink/ and renamed {} change data file(s) from .openspec.yaml to .meta.yaml",
        report.renamed
    )
}

/// `tools` arrives already resolved and validated by `cmd_init` — filesystem and remote
/// init share that one entry, so a remote checkout is bootstrapped from the same selection
/// rules (and the same non-empty guarantee) as a local one.
fn cmd_init_remote(
    a: &InitArgs,
    root: &std::path::Path,
    display_base: &str,
    tools: &[core::skills::Tool],
) -> Result<()> {
    let Some(url) = a.url.as_deref() else {
        bail!("--store remote requires --url <project-scoped url>");
    };
    if a.dir.is_some() {
        bail!("--dir has no meaning with --store remote (documents live on the server)");
    }
    core::init::init_remote(root, tools, a.force, url, a.repo.as_deref())?;
    println!("{} Initialized at {display_base} (remote store)", color::green("✓"));
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    println!("Generated files for: {}", names.join(", "));
    // Validate the declared repo now when credentials exist; defer otherwise
    // (offline init must not block — the first verb still validates).
    validate_or_defer(root, url, a.repo.as_deref())?;
    // 這裡產生的技能來自本機的範本（還沒登入的人也要有技能可用）。
    println!("! After you log in (`speclink auth login`), run `speclink update` to get the skills this server provides");
    Ok(())
}

#[cfg(test)]
mod init_tools_tests {
    //! `init` 的工具解析入口與 TUI 多選器單元測試。
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use speclink_core::skills::Tool;
    use speclink_verbs::render::Paint;
    use std::cell::Cell;

    fn resolve_without_picker(spec: Option<&str>, interactive: bool) -> (Result<Vec<Tool>>, bool) {
        let called = Cell::new(false);
        let got = resolve_init_tools(spec, interactive, || {
            called.set(true);
            bail!("picker should not be invoked")
        });
        (got, called.get())
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    /// 缺選集的單行訊息：`pi` 是 `copilot` 的子字串，逐 token 比對測不到漏列，故比全文。
    const ALL_TOOLS_HINT: &str = "no AI tool selected — pass --tools with one or more of claude, codex, copilot, pi, cursor, agy, omp (comma-separated, e.g. --tools claude,omp)";

    fn single_line_error(got: Result<Vec<Tool>>) -> String {
        let message = got.expect_err("must fail").to_string();
        assert_eq!(message.lines().count(), 1, "must be a single line: {message}");
        message
    }

    #[test]
    fn explicit_tools_are_used_without_prompting() {
        let (got, called) = resolve_without_picker(Some("claude,codex"), true);
        assert_eq!(got.expect("valid selection"), vec![Tool::Claude, Tool::Codex]);
        assert!(!called, "顯式 --tools 不得啟動 TUI");
    }

    #[test]
    fn explicit_duplicates_collapse_to_one_entry() {
        let (got, _) = resolve_without_picker(Some("codex, codex"), false);
        assert_eq!(got.expect("valid selection"), vec![Tool::Codex]);
    }

    #[test]
    fn explicit_empty_selection_is_rejected_naming_the_flag_and_values() {
        let (got, _) = resolve_without_picker(Some("  ,  "), true);
        assert_eq!(single_line_error(got), ALL_TOOLS_HINT);
    }

    #[test]
    fn explicit_unknown_tool_names_the_offender() {
        let (got, _) = resolve_without_picker(Some("claude,vscode"), true);
        assert!(single_line_error(got).contains("vscode"));
    }

    #[test]
    fn explicit_tools_accept_copilot_in_any_case_and_combination() {
        let cases: [(&str, &[Tool]); 5] = [
            ("copilot", &[Tool::Copilot]),
            ("Copilot", &[Tool::Copilot]),
            ("claude,copilot", &[Tool::Claude, Tool::Copilot]),
            ("copilot,copilot", &[Tool::Copilot]),
            (
                "claude,codex,copilot",
                &[Tool::Claude, Tool::Codex, Tool::Copilot],
            ),
        ];
        for (spec, want) in cases {
            let (got, called) = resolve_without_picker(Some(spec), true);
            assert_eq!(got.expect(spec), want, "--tools {spec}");
            assert!(!called, "顯式 --tools 不得啟動 TUI");
        }
    }

    #[test]
    fn explicit_tools_accept_pi_and_cursor_in_any_case_and_combination() {
        let cases: [(&str, &[Tool]); 17] = [
            ("pi", &[Tool::Pi]),
            ("Pi", &[Tool::Pi]),
            ("Cursor", &[Tool::Cursor]),
            ("CURSOR", &[Tool::Cursor]),
            ("cursor,cursor", &[Tool::Cursor]),
            ("codex,pi", &[Tool::Codex, Tool::Pi]),
            ("agy", &[Tool::Agy]),
            ("AGY", &[Tool::Agy]),
            ("antigravity", &[Tool::Agy]),
            ("agy,agy", &[Tool::Agy]),
            ("codex,agy", &[Tool::Codex, Tool::Agy]),
            ("omp", &[Tool::Omp]),
            ("OMP", &[Tool::Omp]),
            ("oh-my-pi", &[Tool::Omp]),
            ("omp,oh-my-pi", &[Tool::Omp]),
            ("pi,omp", &[Tool::Pi, Tool::Omp]),
            (
                "claude,codex,copilot,pi,cursor,agy,omp",
                &[Tool::Claude, Tool::Codex, Tool::Copilot, Tool::Pi, Tool::Cursor, Tool::Agy, Tool::Omp],
            ),
        ];
        for (spec, want) in cases {
            let (got, called) = resolve_without_picker(Some(spec), true);
            assert_eq!(got.expect(spec), want, "--tools {spec}");
            assert!(!called, "顯式 --tools 不得啟動 TUI");
        }
    }

    #[test]
    fn explicit_unknown_tool_lists_the_supported_set() {
        for unknown in ["github-copilot", "cursor-agent", "pi-coding-agent"] {
            let (got, _) = resolve_without_picker(Some(unknown), true);
            assert_eq!(
                single_line_error(got),
                format!("unknown tool: {unknown} (supported: claude, codex, copilot, pi, cursor, agy, omp)")
            );
        }
    }

    #[test]
    fn non_interactive_without_tools_fails_without_invoking_picker() {
        let (got, called) = resolve_without_picker(None, false);
        assert_eq!(single_line_error(got), ALL_TOOLS_HINT);
        assert!(!called, "非互動終端不得啟動 TUI");
    }

    #[test]
    fn tui_navigate_and_toggle_selects_in_canonical_order() {
        let tmp = std::env::temp_dir().join(format!("speclink-tui-nav-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let ctx = init_tui::collect_context(&tmp, ".", "fs", None);
        let mut picker = init_tui::ToolPicker::new(ctx.options);

        // Down -> Claude (1), Space toggles Claude; Up -> Select All, Up wraps -> oh-my-pi (7)
        assert_eq!(picker.handle_key(key(KeyCode::Down)), init_tui::PickerStatus::Active);
        assert_eq!(picker.handle_key(key(KeyCode::Char(' '))), init_tui::PickerStatus::Active);
        assert_eq!(picker.handle_key(key(KeyCode::Up)), init_tui::PickerStatus::Active);
        assert_eq!(picker.handle_key(key(KeyCode::Up)), init_tui::PickerStatus::Active);
        assert_eq!(picker.handle_key(key(KeyCode::Char(' '))), init_tui::PickerStatus::Active);
        assert_eq!(picker.handle_key(key(KeyCode::Enter)), init_tui::PickerStatus::Confirmed);
        assert_eq!(picker.selected_tools(), vec![Tool::Claude, Tool::Omp]);

        let omp_opt = init_tui::collect_context(&tmp, ".", "fs", None)
            .options
            .into_iter()
            .find(|o| o.tool == Tool::Omp)
            .expect("oh-my-pi option present in TUI picker");
        assert_eq!(omp_opt.label, "oh-my-pi");
        assert_eq!(omp_opt.skills_dir, ".omp/skills");
        assert_eq!(omp_opt.invoke_hint, "/skill:speclink-*");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn tui_search_filters_and_ctrl_a_toggles_matching_subset() {
        let tmp = std::env::temp_dir().join(format!("speclink-tui-filter-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let ctx = init_tui::collect_context(&tmp, ".", "fs", None);
        let mut picker = init_tui::ToolPicker::new(ctx.options);

        // Typing "co" filters to Claude Code, Codex, and GitHub Copilot (not Pi / Cursor)
        picker.handle_key(key(KeyCode::Char('c')));
        picker.handle_key(key(KeyCode::Char('o')));
        let visible: Vec<Tool> = picker
            .filtered_indices()
            .into_iter()
            .map(|i| Tool::ALL[i])
            .collect();
        assert_eq!(visible, vec![Tool::Claude, Tool::Codex, Tool::Copilot]);

        // Typing "p" -> "cop" narrows strictly to GitHub Copilot
        picker.handle_key(key(KeyCode::Char('p')));
        let narrow: Vec<Tool> = picker
            .filtered_indices()
            .into_iter()
            .map(|i| Tool::ALL[i])
            .collect();
        assert_eq!(narrow, vec![Tool::Copilot]);

        // Backspace back to "co", then Ctrl+A selects the matching subset cleanly
        picker.handle_key(key(KeyCode::Backspace));
        picker.handle_key(ctrl('a'));
        assert_eq!(picker.selected_tools(), vec![Tool::Claude, Tool::Codex, Tool::Copilot]);

        // Second Ctrl+A clears matching subset
        picker.handle_key(ctrl('a'));
        assert!(picker.selected_tools().is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn tui_empty_enter_warns_inline_and_stays_active_until_selected() {
        let tmp = std::env::temp_dir().join(format!("speclink-tui-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let ctx = init_tui::collect_context(&tmp, ".", "fs", None);
        let mut picker = init_tui::ToolPicker::new(ctx.options);

        // Pressing Enter with 0 selected keeps picker open and renders validation warning
        assert_eq!(picker.handle_key(key(KeyCode::Enter)), init_tui::PickerStatus::Active);
        let rendered = picker.render_lines(Paint::new(false), 80).join("\n");
        assert!(rendered.contains("Pick at least one tool"), "{rendered}");

        // Space on cursor=0 ("Select All") selects all 5, then Enter confirms
        picker.handle_key(key(KeyCode::Char(' ')));
        assert_eq!(picker.handle_key(key(KeyCode::Enter)), init_tui::PickerStatus::Confirmed);
        assert_eq!(picker.selected_tools(), Tool::ALL.to_vec());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn tui_esc_clears_search_first_then_cancels_and_ctrl_c_cancels_immediately() {
        let tmp = std::env::temp_dir().join(format!("speclink-tui-cancel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let ctx = init_tui::collect_context(&tmp, ".", "fs", None);
        let mut picker = init_tui::ToolPicker::new(ctx.options);

        picker.handle_key(key(KeyCode::Char('p')));
        assert_eq!(picker.handle_key(key(KeyCode::Esc)), init_tui::PickerStatus::Active);
        assert_eq!(picker.filtered_indices().len(), Tool::ALL.len());
        assert_eq!(picker.handle_key(key(KeyCode::Esc)), init_tui::PickerStatus::Cancelled);

        picker.handle_key(key(KeyCode::Char('c')));
        assert_eq!(picker.handle_key(ctrl('c')), init_tui::PickerStatus::Cancelled);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn tui_prechecks_configured_tools_and_no_color_emits_zero_ansi() {
        let tmp = std::env::temp_dir().join(format!("speclink-tui-nocolor-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join(".speclink.yaml"), "tools:\n  - claude\n  - pi\n").unwrap();

        let ctx = init_tui::collect_context(&tmp, &tmp.display().to_string(), "fs", None);
        let mut intro_buf = Vec::new();
        init_tui::render_static_intro(&mut intro_buf, &ctx, Paint::new(false), 80).unwrap();
        let intro = String::from_utf8(intro_buf).unwrap();
        assert!(!intro.contains('\x1b'), "no-color intro must have zero ANSI: {intro:?}");

        let picker = init_tui::ToolPicker::new(ctx.options);
        assert_eq!(picker.selected_tools(), vec![Tool::Claude, Tool::Pi]);
        let frame = picker.render_lines(Paint::new(false), 80).join("\n");
        assert!(!frame.contains('\x1b'), "no-color frame must have zero ANSI: {frame:?}");
        assert!(frame.contains("Select All (2/7)"), "{frame}");
        assert!(frame.contains("oh-my-pi") && frame.contains(".omp/skills"), "{frame}");
        assert!(frame.contains("configured"), "{frame}");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
