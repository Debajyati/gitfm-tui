//! Terminal user interface rendering logic with Ratatui.
//!
//! Provides a responsive 3-pane Miller column layout (Repositories -> Directory Contents -> File Preview)
//! modeled after Yazi, along with modal overlays for search, cloning, and help.

use crate::app::App;
use crate::types::{AppMode, CloneMethod, FileType, FocusedPane, Platform};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap,
    },
    Frame,
};

/// Main render entry point that draws the active frame based on application state.
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Platform Tabs
            Constraint::Min(10),   // Miller Columns (Yazi view)
            Constraint::Length(1), // Footer / Status Bar
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_miller_columns(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);

    // Draw modals if active
    match app.mode {
        AppMode::SearchInput => draw_search_modal(f, app),
        AppMode::CloneModal => draw_clone_modal(f, app),
        AppMode::CloningInProgress => draw_cloning_progress_modal(f, app),
        AppMode::CloneFinished => draw_clone_finished_modal(f, app),
        AppMode::HelpModal => draw_help_modal(f),
        AppMode::Normal => {}
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(18), Constraint::Min(20), Constraint::Length(25)])
        .split(area);

    // App Title
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .border_type(BorderType::Rounded);
    let title_p = Paragraph::new(Line::from(vec![
        Span::styled("🚀 gitFM ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("TUI", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ]))
    .alignment(Alignment::Center)
    .block(title_block);
    f.render_widget(title_p, header_chunks[0]);

    // Platform Tabs
    let titles = vec![
        Line::from(vec![
            Span::raw("1 "),
            Span::styled("GitHub", if app.platform == Platform::GitHub { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default() }),
        ]),
        Line::from(vec![
            Span::raw("2 "),
            Span::styled("GitLab", if app.platform == Platform::GitLab { Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD) } else { Style::default() }),
        ]),
    ];

    let selected_tab = match app.platform {
        Platform::GitHub => 0,
        Platform::GitLab => 1,
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Platform (Tab) ")
                .border_type(BorderType::Rounded),
        )
        .select(selected_tab)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, header_chunks[1]);

    // Query badge / Auth status
    let auth_text = match app.platform {
        Platform::GitHub => {
            if app.github_client.is_some() {
                Span::styled("● GitHub Ready", Style::default().fg(Color::Green))
            } else {
                Span::styled("○ Public Mode", Style::default().fg(Color::Yellow))
            }
        }
        Platform::GitLab => {
            if let Some(ref gl) = app.gitlab_client {
                if gl.has_token() {
                    Span::styled("● GitLab Token Set", Style::default().fg(Color::Green))
                } else {
                    Span::styled("○ Public Mode", Style::default().fg(Color::Yellow))
                }
            } else {
                Span::styled("○ Public Mode", Style::default().fg(Color::Yellow))
            }
        }
    };

    let auth_block = Block::default()
        .borders(Borders::ALL)
        .title(" Status ")
        .border_type(BorderType::Rounded);
    let auth_p = Paragraph::new(Line::from(auth_text))
        .alignment(Alignment::Center)
        .block(auth_block);
    f.render_widget(auth_p, header_chunks[2]);
}

fn draw_miller_columns(f: &mut Frame, app: &App, area: Rect) {
    // 3 Miller columns: [Left: Repositories] | [Middle: Directory Contents] | [Right: Preview]
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ])
        .split(area);

    draw_repo_column(f, app, columns[0]);
    draw_file_column(f, app, columns[1]);
    draw_preview_column(f, app, columns[2]);
}

fn draw_repo_column(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focused_pane == FocusedPane::RepoList;
    let border_color = if is_focused { Color::Green } else { Color::DarkGray };

    let items: Vec<ListItem> = app
        .repos
        .iter()
        .enumerate()
        .map(|(idx, repo)| {
            let is_selected = idx == app.repo_selected_index;
            let icon = if is_selected { "▶ " } else { "  " };

            let lang_badge = repo
                .language
                .as_deref()
                .map(|l| format!(" [{}]", l))
                .unwrap_or_default();

            let line = Line::from(vec![
                Span::styled(icon, Style::default().fg(Color::Yellow)),
                Span::styled(
                    &repo.name,
                    if is_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Cyan)
                    },
                ),
                Span::styled(
                    format!(" ⭐{} {}", repo.stars, lang_badge),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            let item_style = if is_selected {
                Style::default().bg(Color::Rgb(30, 60, 90))
            } else {
                Style::default()
            };

            ListItem::new(line).style(item_style)
        })
        .collect();

    let title = format!(
        " Repositories ({}) [{}] ",
        app.repos.len(),
        if app.search_query.is_empty() { "none" } else { &app.search_query }
    );

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_type(if is_focused { BorderType::Thick } else { BorderType::Rounded })
                .border_style(Style::default().fg(border_color)),
        );

    f.render_widget(list, area);
}

fn draw_file_column(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focused_pane == FocusedPane::FileList;
    let border_color = if is_focused { Color::Green } else { Color::DarkGray };

    let items: Vec<ListItem> = app
        .files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let is_selected = idx == app.file_selected_index;
            let marker = if is_selected { "▶ " } else { "  " };

            let (type_icon, name_color) = match file.file_type {
                FileType::Directory => ("📁 ", Color::LightBlue),
                FileType::File => ("📄 ", Color::White),
                FileType::Symlink => ("🔗 ", Color::Magenta),
                FileType::Submodule => ("📦 ", Color::Yellow),
            };

            let size_str = if let Some(bytes) = file.size {
                format_file_size(bytes)
            } else {
                String::new()
            };

            let line = Line::from(vec![
                Span::styled(marker, Style::default().fg(Color::Yellow)),
                Span::raw(type_icon),
                Span::styled(
                    &file.name,
                    if is_selected {
                        Style::default()
                            .fg(name_color)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(name_color)
                    },
                ),
                Span::styled(format!(" {}", size_str), Style::default().fg(Color::DarkGray)),
            ]);

            let item_style = if is_selected {
                Style::default().bg(Color::Rgb(40, 50, 70))
            } else {
                Style::default()
            };

            ListItem::new(line).style(item_style)
        })
        .collect();

    let path_display = if app.current_path.is_empty() {
        "/ (root)".to_string()
    } else {
        format!("/{}", app.current_path_string())
    };

    let title = format!(" Files: {} ({}) ", path_display, app.files.len());

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_type(if is_focused { BorderType::Thick } else { BorderType::Rounded })
                .border_style(Style::default().fg(border_color)),
        );

    f.render_widget(list, area);
}

fn draw_preview_column(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focused_pane == FocusedPane::Preview;
    let border_color = if is_focused { Color::Green } else { Color::DarkGray };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Preview / Details ")
        .border_type(if is_focused { BorderType::Thick } else { BorderType::Rounded })
        .border_style(Style::default().fg(border_color));

    if let Some(ref text) = app.preview_content {
        // File Content Preview
        let lines: Vec<Line> = text
            .lines()
            .skip(app.preview_scroll)
            .map(|l| Line::from(Span::styled(l, Style::default().fg(Color::LightCyan))))
            .collect();

        let p = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(p, area);
        return;
    }

    if let Some(file) = app.selected_file() {
        if file.file_type == FileType::Directory {
            let info = vec![
                Line::from(vec![
                    Span::styled("Folder: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::styled(&file.path, Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Action: ", Style::default().fg(Color::Cyan)),
                    Span::raw("Press 'l' or 'Enter' to step inside this directory."),
                ]),
                Line::from(vec![
                    Span::styled("Sparse Clone: ", Style::default().fg(Color::Green)),
                    Span::raw("Press 'c' to clone ONLY this subfolder!"),
                ]),
            ];

            let p = Paragraph::new(info).block(block);
            f.render_widget(p, area);
            return;
        }
    }

    if let Some(repo) = app.selected_repo() {
        // Repository Overview Preview
        let desc = repo.description.as_deref().unwrap_or("No description provided.");
        let lang = repo.language.as_deref().unwrap_or("Unknown");

        let lines = vec![
            Line::from(vec![
                Span::styled("Repo: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(&repo.full_name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Description:\n", Style::default().fg(Color::Cyan)),
                Span::styled(desc, Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Stars: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("⭐ {}    ", repo.stars)),
                Span::styled("Forks: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("🍴 {}", repo.forks)),
            ]),
            Line::from(vec![
                Span::styled("Default Branch: ", Style::default().fg(Color::Magenta)),
                Span::styled(&repo.default_branch, Style::default().fg(Color::LightMagenta)),
            ]),
            Line::from(vec![
                Span::styled("Language: ", Style::default().fg(Color::Blue)),
                Span::styled(lang, Style::default().fg(Color::LightBlue)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Clone URL: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&repo.clone_url, Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("▶ Press 'l' or 'Enter' to explore repository files", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled("▶ Press 'c' to open Clone Dialog", Style::default().fg(Color::Yellow)),
            ]),
        ];

        let p = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: true });
        f.render_widget(p, area);
    } else {
        let p = Paragraph::new("No repository selected. Press '/' to search.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(block);
        f.render_widget(p, area);
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let (msg_color, msg_prefix) = if let Some(ref err) = app.error_message {
        (Color::Red, format!("❌ {}", err))
    } else if app.is_loading {
        (Color::Yellow, format!("⏳ {}", app.status_message))
    } else {
        (Color::White, app.status_message.clone())
    };

    let key_hints = Line::from(vec![
        Span::styled(format!(" {} ", msg_prefix), Style::default().fg(msg_color)),
        Span::raw(" | "),
        Span::styled("/", Style::default().fg(Color::Yellow)),
        Span::raw(" Search  "),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" Platform  "),
        Span::styled("h/j/k/l", Style::default().fg(Color::Yellow)),
        Span::raw(" Nav  "),
        Span::styled("c", Style::default().fg(Color::Yellow)),
        Span::raw(" Clone  "),
        Span::styled("r", Style::default().fg(Color::Yellow)),
        Span::raw(" Refresh  "),
        Span::styled("?", Style::default().fg(Color::Yellow)),
        Span::raw(" Help  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" Quit "),
    ]);

    let footer = Paragraph::new(key_hints)
        .style(Style::default().bg(Color::Rgb(20, 20, 25)));
    f.render_widget(footer, area);
}

// -------------------------------------------------------------
// Modals
// -------------------------------------------------------------

fn draw_search_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 25, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Search {} Repositories ", app.platform.name()))
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(2)])
        .split(inner);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(" Query ")
        .border_style(Style::default().fg(Color::Green));

    let input_p = Paragraph::new(app.search_input.clone())
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
        .block(input_block);
    f.render_widget(input_p, chunks[0]);

    let help_text = vec![
        Line::from("Press [Enter] to Search  •  [Esc] to Cancel"),
        Line::from(vec![
            Span::styled("Tip: ", Style::default().fg(Color::Yellow)),
            Span::raw("Search by keyword, topic, or username (e.g. 'ratatui' or 'user:facebook')."),
        ]),
    ];
    let help_p = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(help_p, chunks[1]);
}

fn draw_clone_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 70, f.area());
    f.render_widget(Clear, area);

    let repo_name = app
        .selected_repo()
        .map(|r| r.name.as_str())
        .unwrap_or("repository");

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Clone Repository: {} ", repo_name))
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Clone Method Selector
            Constraint::Length(3), // Target Directory Input
            Constraint::Length(3), // Branch Input
            Constraint::Min(4),    // Description & Controls
        ])
        .split(inner);

    // 1. Clone Method Selector
    let methods = CloneMethod::all();
    let method_items: Vec<ListItem> = methods
        .iter()
        .map(|m| {
            let is_selected = *m == app.clone_method;
            let marker = if is_selected { "● " } else { "○ " };
            let line = Line::from(vec![
                Span::styled(marker, if is_selected { Style::default().fg(Color::Green) } else { Style::default().fg(Color::DarkGray) }),
                Span::styled(
                    m.name(),
                    if is_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let method_border_color = if app.clone_focused_field == 0 { Color::Green } else { Color::DarkGray };
    let method_list = List::new(method_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" [1] Clone Method (Use Left/Right or Up/Down) ")
            .border_style(Style::default().fg(method_border_color)),
    );
    f.render_widget(method_list, rows[0]);

    // 2. Target Directory Input
    let dir_border_color = if app.clone_focused_field == 1 { Color::Green } else { Color::DarkGray };
    let dir_input = Paragraph::new(app.clone_dir_input.clone()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" [2] Target Local Directory ")
            .border_style(Style::default().fg(dir_border_color)),
    );
    f.render_widget(dir_input, rows[1]);

    // 3. Branch Input
    let branch_border_color = if app.clone_focused_field == 2 { Color::Green } else { Color::DarkGray };
    let branch_input = Paragraph::new(app.clone_branch_input.clone()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" [3] Branch (Leave blank for default) ")
            .border_style(Style::default().fg(branch_border_color)),
    );
    f.render_widget(branch_input, rows[2]);

    // 4. Method description and instructions
    let desc = app.clone_method.description();
    let instructions = vec![
        Line::from(vec![
            Span::styled("Method Details: ", Style::default().fg(Color::Yellow)),
            Span::raw(desc),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Tab] ", Style::default().fg(Color::Cyan)),
            Span::raw("Next Field  •  "),
            Span::styled("[Enter] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("START CLONING  •  "),
            Span::styled("[Esc] ", Style::default().fg(Color::Red)),
            Span::raw("Cancel"),
        ]),
    ];

    let desc_p = Paragraph::new(instructions)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::Gray));
    f.render_widget(desc_p, rows[3]);
}

fn draw_cloning_progress_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 30, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Cloning In Progress ⏳ ")
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow));

    let p = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled("Executing Git Clone command...", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled(format!("Target: ./{}", app.clone_dir_input), Style::default().fg(Color::White))),
        Line::from(Span::styled(format!("Method: {}", app.clone_method.name()), Style::default().fg(Color::Yellow))),
        Line::from(""),
        Line::from(Span::styled("Please wait while git fetches the repository data...", Style::default().fg(Color::DarkGray))),
    ])
    .alignment(Alignment::Center)
    .block(block);

    f.render_widget(p, area);
}

fn draw_clone_finished_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 60, f.area());
    f.render_widget(Clear, area);

    let (border_color, title) = if app.clone_success {
        (Color::Green, " ✔ Cloning Succeeded! ")
    } else {
        (Color::Red, " ✖ Cloning Failed ")
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(inner);

    let output_lines: Vec<Line> = app
        .clone_output
        .lines()
        .map(|l| Line::from(Span::styled(l, Style::default().fg(if app.clone_success { Color::LightGreen } else { Color::LightRed }))))
        .collect();

    let output_p = Paragraph::new(output_lines)
        .wrap(Wrap { trim: false });
    f.render_widget(output_p, chunks[0]);

    let dismiss = Paragraph::new("Press [Enter] or [Esc] to return to File Manager")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    f.render_widget(dismiss, chunks[1]);
}

fn draw_help_modal(f: &mut Frame) {
    let area = centered_rect(65, 70, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" gitFM TUI - Cheatsheet & Keybindings ")
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));

    let content = vec![
        Line::from(Span::styled("Navigation (Yazi / Vim-style):", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("  j / Down       : Move selection down"),
        Line::from("  k / Up         : Move selection up"),
        Line::from("  l / Right      : Enter folder / open directory in Miller column"),
        Line::from("  h / Left       : Go up to parent directory / back to repo list"),
        Line::from("  Enter          : Open folder or view file preview"),
        Line::from(""),
        Line::from(Span::styled("Cloning & Actions:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("  c              : Open Clone Dialog (supports Sparse for current folder!)"),
        Line::from("  / or s         : Search GitHub or GitLab repositories"),
        Line::from("  Tab            : Toggle between GitHub and GitLab platforms"),
        Line::from("  r              : Refresh directory contents / repositories"),
        Line::from("  ?              : Toggle this help menu"),
        Line::from("  q / Esc        : Back / Close modal / Quit application"),
        Line::from(""),
        Line::from(Span::styled("Cloning Methods:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("  • Normal       : Standard full git clone"),
        Line::from("  • Shallow      : --depth 1 (only latest commit)"),
        Line::from("  • Blobless     : --filter=blob:none (blobs fetched on-demand)"),
        Line::from("  • Treeless     : --filter=tree:0 (trees & blobs fetched on-demand)"),
        Line::from("  • Sparse       : Clones only the currently highlighted subfolder!"),
    ];

    let p = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Formats raw byte counts into human-readable B, KB, or MB representations.
pub fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1023), "1023 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_file_size(5 * 1024 * 1024 + 512 * 1024), "5.5 MB");
    }

    #[test]
    fn test_centered_rect() {
        let parent = Rect::new(0, 0, 100, 50);
        let popup = centered_rect(60, 40, parent);
        assert_eq!(popup.width, 60);
        assert_eq!(popup.height, 20); // 40% of 50 is 20
        assert_eq!(popup.x, 20);      // (100 - 60) / 2 = 20
        assert_eq!(popup.y, 15);      // (50 - 20) / 2 = 15
    }
}
