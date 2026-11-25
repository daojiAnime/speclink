//! Interactive Clack / `npx skills` style multi-select TUI for `speclink init`.
//!
//! Renders an inline monospace workspace selector on `stderr` (keeping `stdout`
//! reserved for machine/summary lines) using `crossterm` raw mode with a drop
//! guard so cursor visibility and terminal state are always restored on exit,
//! `Esc`, `Ctrl+C`, or panic.

use anyhow::{bail, Result};
use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{self, Clear, ClearType};
use speclink_core as core;
use speclink_core::config::ToolEntry;
use speclink_core::skills::Tool;
use speclink_verbs::render::Paint;
use std::io::Write;
use std::path::Path;

const BANNER_COLORS: [u8; 5] = [255, 252, 248, 243, 239];
const BANNER_ROWS: [&str; 5] = [
    "█████ █████ █████ █████ █     ███ █   █ █   █",
    "█     █   █ █     █     █      █  ██  █ █  █ ",
    "█████ █████ ████  █     █      █  █ █ █ ███  ",
    "    █ █     █     █     █      █  █  ██ █  █ ",
    "█████ █     █████ █████ █████ ███ █   █ █   █",
];

#[derive(Debug, Clone)]
pub(crate) struct InitContext {
    pub display_project: String,
    pub store_summary: String,
    pub options: Vec<ToolOption>,
}

#[derive(Debug, Clone)]
pub(crate) struct ToolOption {
    pub tool: Tool,
    pub label: &'static str,
    pub skills_dir: &'static str,
    pub invoke_hint: &'static str,
    pub description: &'static str,
    pub configured: bool,
    pub detected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PickerStatus {
    Active,
    Confirmed,
    Cancelled,
}

pub(crate) struct ToolPicker {
    options: Vec<ToolOption>,
    selected: Vec<bool>,
    search: String,
    cursor: usize,
    error: Option<String>,
}

struct RawModeGuard;

impl RawModeGuard {
    fn enter(out: &mut impl Write) -> std::io::Result<Self> {
        terminal::enable_raw_mode()?;
        let _ = execute!(out, Hide);
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let mut stderr = std::io::stderr();
        let _ = execute!(stderr, Show);
        let _ = terminal::disable_raw_mode();
    }
}

pub(crate) fn collect_context(root: &Path, display_base: &str, store: &str, remote_url: Option<&str>) -> InitContext {
    let existing_cfg = core::config::AppConfig::load(&root.join(".speclink.yaml")).ok();
    let has_app_cfg = existing_cfg.is_some();
    let configured_tools: Vec<Tool> = existing_cfg
        .as_ref()
        .map(|cfg| {
            cfg.tools
                .iter()
                .filter_map(|entry| match entry {
                    ToolEntry::Builtin(name) => Tool::parse(name),
                    ToolEntry::Descriptor(_) => None,
                })
                .collect()
        })
        .unwrap_or_default();

    let display_project = match Path::new(display_base).canonicalize().ok() {
        Some(abs) => shorten_home(&abs),
        None => shorten_home(root),
    };

    let store_summary = if store == "remote" {
        match remote_url {
            Some(url) => format!("remote ({url})"),
            None => "remote".to_string(),
        }
    } else {
        "local filesystem (speclink/)".to_string()
    };

    let options = vec![
        build_option(
            root,
            has_app_cfg,
            &configured_tools,
            Tool::Claude,
            "Claude Code",
            "/speclink-*",
            "Generate .claude/skills/ (/speclink-*) with optional Herdr hook tracking.",
            &["claude"],
        ),
        build_option(
            root,
            has_app_cfg,
            &configured_tools,
            Tool::Codex,
            "Codex",
            "$speclink-*",
            "Generate .agents/skills/ ($speclink-*) for OpenAI Codex CLI agents.",
            &["codex"],
        ),
        build_option(
            root,
            has_app_cfg,
            &configured_tools,
            Tool::Copilot,
            "GitHub Copilot",
            "/speclink-*",
            "Generate .github/skills/ (/speclink-*) for Copilot coding agents.",
            &["copilot"],
        ),
        build_option(
            root,
            has_app_cfg,
            &configured_tools,
            Tool::Pi,
            "Pi",
            "/skill:speclink-*",
            "Generate .pi/skills/ (/skill:speclink-*) for Pi coding agent.",
            &["pi"],
        ),
        build_option(
            root,
            has_app_cfg,
            &configured_tools,
            Tool::Cursor,
            "Cursor",
            "/speclink-*",
            "Generate .cursor/skills/ (/speclink-*) for Cursor AI editor & CLI.",
            &["agent", "cursor"],
        ),
        build_option(
            root,
            has_app_cfg,
            &configured_tools,
            Tool::Agy,
            "Antigravity",
            "/speclink-*",
            "Generate .agents/skills/ (/speclink-*) for Antigravity IDE & agy CLI.",
            &["agy"],
        ),
    ];

    InitContext { display_project, store_summary, options }
}

fn build_option(
    root: &Path,
    has_app_cfg: bool,
    configured_tools: &[Tool],
    tool: Tool,
    label: &'static str,
    invoke_hint: &'static str,
    description: &'static str,
    bins: &[&str],
) -> ToolOption {
    let skills_dir = tool.skills_dir();
    let configured = if has_app_cfg {
        configured_tools.contains(&tool)
    } else {
        let skill_path = root.join(skills_dir).join("speclink-propose/SKILL.md");
        match std::fs::read_to_string(&skill_path) {
            Ok(content) if skills_dir == ".agents/skills" => {
                let is_agy = content.contains("--agent agy");
                if tool == Tool::Agy { is_agy } else { !is_agy }
            }
            Ok(_) => true,
            Err(_) => false,
        }
    };
    let detected = bins.iter().any(|bin| command_in_path(bin));
    ToolOption {
        tool,
        label,
        skills_dir,
        invoke_hint,
        description,
        configured,
        detected,
    }
}

fn command_in_path(cmd: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| {
        let candidate = dir.join(cmd);
        candidate.is_file()
    })
}

fn shorten_home(path: &Path) -> String {
    let s = path.display().to_string();
    if let Some(home) = std::env::var_os("HOME").map(|h| Path::new(&h).display().to_string()) {
        if s == home {
            return "~".to_string();
        }
        if let Some(rest) = s.strip_prefix(&(home + "/")) {
            return format!("~/{rest}");
        }
    }
    s
}

impl ToolPicker {
    pub(crate) fn new(options: Vec<ToolOption>) -> Self {
        let selected = options.iter().map(|opt| opt.configured).collect();
        Self {
            options,
            selected,
            search: String::new(),
            cursor: 0,
            error: None,
        }
    }

    pub(crate) fn filtered_indices(&self) -> Vec<usize> {
        let q = self.search.trim().to_ascii_lowercase();
        if q.is_empty() {
            return (0..self.options.len()).collect();
        }
        self.options
            .iter()
            .enumerate()
            .filter(|(_, opt)| {
                opt.label.to_ascii_lowercase().contains(&q)
                    || opt.tool.name().to_ascii_lowercase().contains(&q)
                    || opt.skills_dir.to_ascii_lowercase().contains(&q)
                    || opt.invoke_hint.to_ascii_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn row_count(&self, filtered: &[usize]) -> usize {
        if filtered.is_empty() { 0 } else { 1 + filtered.len() }
    }

    fn clamp_cursor(&mut self) {
        let filtered = self.filtered_indices();
        let count = self.row_count(&filtered);
        if count == 0 {
            self.cursor = 0;
        } else if self.cursor >= count {
            self.cursor = count - 1;
        }
    }

    pub(crate) fn selected_tools(&self) -> Vec<Tool> {
        self.options
            .iter()
            .enumerate()
            .filter(|(i, _)| *self.selected.get(*i).unwrap_or(&false))
            .map(|(_, opt)| opt.tool)
            .collect()
    }

    fn toggle_all_filtered(&mut self) {
        let filtered = self.filtered_indices();
        if filtered.is_empty() {
            return;
        }
        let all_checked = filtered.iter().all(|&idx| self.selected[idx]);
        let next = !all_checked;
        for idx in filtered {
            self.selected[idx] = next;
        }
        self.error = None;
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> PickerStatus {
        if key.kind == KeyEventKind::Release {
            return PickerStatus::Active;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);

        if ctrl {
            return match key.code {
                KeyCode::Char('c') => PickerStatus::Cancelled,
                KeyCode::Char('a') => {
                    self.toggle_all_filtered();
                    PickerStatus::Active
                }
                KeyCode::Char('u') => {
                    self.search.clear();
                    self.cursor = 0;
                    self.error = None;
                    PickerStatus::Active
                }
                KeyCode::Char('p') => {
                    self.move_up();
                    PickerStatus::Active
                }
                KeyCode::Char('n') => {
                    self.move_down();
                    PickerStatus::Active
                }
                _ => PickerStatus::Active,
            };
        }

        match key.code {
            KeyCode::Up | KeyCode::BackTab => self.move_up(),
            KeyCode::Down | KeyCode::Tab => self.move_down(),
            KeyCode::Char(' ') if !alt => {
                let filtered = self.filtered_indices();
                if !filtered.is_empty() {
                    if self.cursor == 0 {
                        self.toggle_all_filtered();
                    } else if let Some(&opt_idx) = filtered.get(self.cursor - 1) {
                        self.selected[opt_idx] = !self.selected[opt_idx];
                        self.error = None;
                    }
                }
            }
            KeyCode::Enter => {
                if self.selected.iter().any(|&on| on) {
                    return PickerStatus::Confirmed;
                }
                self.error = Some(
                    "Pick at least one tool (use Space to select, or Ctrl+A to select all)."
                        .to_string(),
                );
            }
            KeyCode::Backspace => {
                if self.search.pop().is_some() {
                    self.cursor = 0;
                    self.error = None;
                    self.clamp_cursor();
                }
            }
            KeyCode::Esc => {
                if !self.search.is_empty() {
                    self.search.clear();
                    self.cursor = 0;
                    self.error = None;
                } else {
                    return PickerStatus::Cancelled;
                }
            }
            KeyCode::Char(c) if !alt && !c.is_control() => {
                self.search.push(c);
                self.cursor = 0;
                self.error = None;
                self.clamp_cursor();
            }
            _ => {}
        }
        PickerStatus::Active
    }

    fn move_up(&mut self) {
        let filtered = self.filtered_indices();
        let count = self.row_count(&filtered);
        if count == 0 {
            self.cursor = 0;
            return;
        }
        self.cursor = if self.cursor == 0 { count - 1 } else { self.cursor - 1 };
    }

    fn move_down(&mut self) {
        let filtered = self.filtered_indices();
        let count = self.row_count(&filtered);
        if count == 0 {
            self.cursor = 0;
            return;
        }
        self.cursor = (self.cursor + 1) % count;
    }

    fn current_description(&self, filtered: &[usize]) -> String {
        if filtered.is_empty() {
            return format!(
                "No tools match '{}'. Press Backspace or Esc to clear filter.",
                self.search
            );
        }
        if self.cursor == 0 {
            let total = filtered.len();
            return format!("Select or clear all {total} AI tool(s). Press Ctrl+A anytime.");
        }
        filtered
            .get(self.cursor - 1)
            .and_then(|&idx| self.options.get(idx))
            .map(|opt| opt.description.to_string())
            .unwrap_or_default()
    }

    pub(crate) fn render_lines(&self, paint: Paint, cols: u16) -> Vec<String> {
        let filtered = self.filtered_indices();
        let max_cols = usize::from(cols.max(36));
        let rail = paint.dim("│");
        let mut lines = Vec::new();

        let prompt_dot = if self.error.is_some() {
            paint.bold_red("▲")
        } else {
            paint.bold_green("◆")
        };
        lines.push(format!(
            "{prompt_dot}  {}",
            paint.bold("Select AI tools to configure")
        ));

        let cursor_glyph = if paint.enabled { "\x1b[7m \x1b[0m" } else { "_" };
        let search_budget = max_cols.saturating_sub(14).max(12);
        let visible_search = tail_plain(&self.search, search_budget);
        lines.push(format!(
            "{rail}  {}{}{}",
            paint.dim("Search: "),
            visible_search,
            cursor_glyph
        ));
        lines.push(format!(
            "{rail}  {}",
            paint.dim("↑↓ move, space select, ctrl+a select all, enter confirm")
        ));
        if let Some(err) = &self.error {
            lines.push(format!(
                "{rail}  {}",
                paint.red(&truncate_plain(err, max_cols.saturating_sub(4)))
            ));
        } else {
            lines.push(rail.clone());
        }

        if filtered.is_empty() {
            lines.push(format!(
                "{rail}  {}",
                paint.dim("(no matching AI tools — press Esc to clear search)")
            ));
        } else {
            let checked_in_filter = filtered.iter().filter(|&&idx| self.selected[idx]).count();
            let all_checked = checked_in_filter == filtered.len();
            let on_all = self.cursor == 0;
            let arrow = if on_all { paint.bold_cyan("❯") } else { " ".to_string() };
            let bullet = if all_checked { paint.bold_green("●") } else { paint.dim("○") };
            let all_label = if on_all {
                underline_or_bold(paint, "Select All")
            } else {
                "Select All".to_string()
            };
            let count_tag = paint.dim(&format!("({checked_in_filter}/{})", filtered.len()));
            lines.push(format!("{rail}  {arrow} {bullet} {all_label} {count_tag}"));

            let sep_len = max_cols.saturating_sub(8).clamp(18, 46);
            lines.push(format!(
                "{rail}    {}",
                paint.dim(&"─".repeat(sep_len))
            ));

            for (pos, &opt_idx) in filtered.iter().enumerate() {
                let opt = &self.options[opt_idx];
                let checked = self.selected[opt_idx];
                let active = self.cursor == pos + 1;

                let ptr = if active { paint.bold_cyan("❯") } else { " ".to_string() };
                let mark = if checked { paint.bold_green("●") } else { paint.dim("○") };
                let pad = " ".repeat(15usize.saturating_sub(opt.label.chars().count()));
                let styled_name = if active {
                    underline_or_bold(paint, opt.label)
                } else if checked {
                    opt.label.to_string()
                } else {
                    paint.dim(opt.label)
                };
                let label_styled = format!("{styled_name}{pad}");

                let status_badge = if opt.configured {
                    format!("  {}", paint.cyan("· configured"))
                } else if opt.detected {
                    format!("  {}", paint.green("· installed"))
                } else {
                    String::new()
                };

                let meta = if max_cols >= 62 {
                    format!("  {}{}", paint.dim(opt.skills_dir), status_badge)
                } else if !status_badge.is_empty() && max_cols >= 48 {
                    status_badge
                } else {
                    String::new()
                };

                lines.push(format!("{rail}  {ptr} {mark} {label_styled}{meta}"));
            }
        }

        lines.push(rail.clone());
        lines.push(format!("{rail}  {}", paint.dim("Description")));
        let desc = self.current_description(&filtered);
        for wrapped in wrap_words(&desc, max_cols.saturating_sub(5).max(20)) {
            lines.push(format!("{rail}  {}", paint.dim(&wrapped)));
        }
        lines.push(paint.dim("└"));
        lines
    }
}

fn underline_or_bold(paint: Paint, s: &str) -> String {
    if paint.enabled {
        format!("\x1b[1;4m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn wrap_words(text: &str, max_width: usize) -> Vec<String> {
    let mut rows = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let wlen = word.chars().count();
        let clen = current.chars().count();
        if current.is_empty() {
            current.push_str(word);
        } else if clen + 1 + wlen <= max_width {
            current.push(' ');
            current.push_str(word);
        } else {
            rows.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        rows.push(current);
    }
    if rows.is_empty() {
        rows.push(String::new());
    }
    rows
}

fn truncate_plain(text: &str, max_width: usize) -> String {
    let count = text.chars().count();
    if count <= max_width {
        return text.to_string();
    }
    if max_width <= 1 {
        return "…".to_string();
    }
    let mut s: String = text.chars().take(max_width - 1).collect();
    s.push('…');
    s
}

fn tail_plain(text: &str, max_width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_width {
        return text.to_string();
    }
    if max_width <= 1 {
        return "…".to_string();
    }
    let start = chars.len() - (max_width - 1);
    let mut s = String::from("…");
    s.extend(&chars[start..]);
    s
}

pub(crate) fn render_static_intro(out: &mut impl Write, ctx: &InitContext, paint: Paint, cols: u16) -> Result<()> {
    let rail = paint.dim("│");
    writeln!(out)?;
    if paint.enabled && cols >= 50 {
        for (row, color_idx) in BANNER_ROWS.iter().zip(BANNER_COLORS.iter()) {
            writeln!(out, "  \x1b[38;5;{color_idx}m{row}\x1b[0m")?;
        }
        writeln!(out)?;
    }

    let badge = if paint.enabled {
        "\x1b[46;30m speclink \x1b[0m".to_string()
    } else {
        "[speclink]".to_string()
    };
    let top_corner = paint.dim("┌");
    writeln!(out, "{top_corner}  {badge}")?;
    writeln!(out, "{rail}")?;

    let diamond = paint.green("◇");
    let max_path = usize::from(cols).saturating_sub(14).max(20);
    let proj = truncate_plain(&ctx.display_project, max_path);
    writeln!(out, "{diamond}  {}  {proj}", paint.dim("Project"))?;
    writeln!(out, "{rail}")?;
    let store = truncate_plain(&ctx.store_summary, max_path);
    writeln!(out, "{diamond}  {}    {store}", paint.dim("Store"))?;
    writeln!(out, "{rail}")?;
    out.flush()?;
    Ok(())
}

pub(crate) fn run_tui_tool_picker(
    root: &Path,
    display_base: &str,
    store: &str,
    remote_url: Option<&str>,
    paint: Paint,
    out: &mut impl Write,
) -> Result<Vec<Tool>> {
    let ctx = collect_context(root, display_base, store, remote_url);
    let (mut cols, mut term_rows) = terminal::size().unwrap_or((80, 24));
    render_static_intro(out, &ctx, paint, cols)?;

    let mut picker = ToolPicker::new(ctx.options);
    let _guard = RawModeGuard::enter(out)?;
    let mut drawn_lines: usize = 0;

    draw_frame(out, &picker, paint, cols, term_rows, &mut drawn_lines)?;

    loop {
        match event::read()? {
            Event::Resize(new_cols, new_rows) => {
                cols = new_cols;
                term_rows = new_rows;
                draw_frame(out, &picker, paint, cols, term_rows, &mut drawn_lines)?;
            }
            Event::Key(key) => {
                match picker.handle_key(key) {
                    PickerStatus::Active => {
                        draw_frame(out, &picker, paint, cols, term_rows, &mut drawn_lines)?;
                    }
                    PickerStatus::Confirmed => {
                        clear_frame(out, drawn_lines, term_rows)?;
                        drop(_guard);
                        let chosen = picker.selected_tools();
                        let names: Vec<&str> = chosen.iter().map(|t| t.name()).collect();
                        writeln!(
                            out,
                            "{}  {}  {}",
                            paint.green("◇"),
                            paint.dim("AI tools"),
                            names.join(", ")
                        )?;
                        writeln!(out, "{}", paint.dim("└"))?;
                        writeln!(out)?;
                        out.flush()?;
                        return Ok(chosen);
                    }
                    PickerStatus::Cancelled => {
                        clear_frame(out, drawn_lines, term_rows)?;
                        drop(_guard);
                        writeln!(
                            out,
                            "{}  {}",
                            paint.red("■"),
                            paint.dim("Cancelled — no files written")
                        )?;
                        writeln!(out, "{}", paint.dim("└"))?;
                        out.flush()?;
                        bail!("no AI tool selected — init cancelled");
                    }
                }
            }
            _ => {}
        }
    }
}

fn clear_frame(out: &mut impl Write, drawn_lines: usize, term_rows: u16) -> Result<()> {
    let up = drawn_lines.min(usize::from(term_rows.saturating_sub(1)).max(1));
    if up > 0 {
        write!(out, "\x1b[{up}A\r")?;
    } else {
        write!(out, "\r")?;
    }
    execute!(out, Clear(ClearType::FromCursorDown))?;
    out.flush()?;
    Ok(())
}

fn draw_frame(
    out: &mut impl Write,
    picker: &ToolPicker,
    paint: Paint,
    cols: u16,
    term_rows: u16,
    drawn_lines: &mut usize,
) -> Result<()> {
    let up = (*drawn_lines).min(usize::from(term_rows.saturating_sub(1)).max(1));
    if up > 0 {
        write!(out, "\x1b[{up}A\r")?;
    } else {
        write!(out, "\r")?;
    }
    execute!(out, Clear(ClearType::FromCursorDown))?;
    let lines = picker.render_lines(paint, cols);
    for line in &lines {
        write!(out, "{line}\r\n")?;
    }
    out.flush()?;
    *drawn_lines = lines.len();
    Ok(())
}
