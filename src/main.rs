//! Entry point for gitFM TUI: initializes terminal raw mode, manages the alternate screen,
//! and dispatches keyboard events to application state and rendering loops.

mod app;
mod banner;
mod config;
mod git;
mod github;
mod gitlab;
mod types;
mod ui;

use app::App;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, stdout};
use std::time::Duration;
use types::{AppMode, CloneMethod, FileType, FocusedPane};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::default();

    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Application error: {:?}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> anyhow::Result<()> {
    loop {
        // Drain any incoming background task results
        while let Ok(res) = app.rx.try_recv() {
            app.handle_task_result(res);
        }

        // Advance spinner animation frame if loading
        app.tick_spinner();

        terminal.draw(|f| ui::draw(f, app))?;

        // Poll events with an 80ms timeout to allow smooth spinner rotation (~12.5 FPS) and responsive renders
        if event::poll(Duration::from_millis(80))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Global abort/exit
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    return Ok(());
                }

                match app.mode {
                    AppMode::Normal => {
                        if app.show_dashboard {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                                KeyCode::Char('?') => {
                                    app.mode = AppMode::HelpModal;
                                }
                                KeyCode::Tab => {
                                    app.switch_platform();
                                }
                                KeyCode::Char('/') | KeyCode::Char('s') | KeyCode::Enter => {
                                    app.search_input.clear();
                                    app.mode = AppMode::SearchInput;
                                }
                                KeyCode::Char('r') => {
                                    app.randomize_banner();
                                }
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('q') => return Ok(()),
                                KeyCode::Char('d') => {
                                    app.show_dashboard = true;
                                    app.status_message = "Press '/' to search, 'Tab' to switch platform, '?' for help".to_string();
                                }
                                KeyCode::Char('?') => {
                                    app.mode = AppMode::HelpModal;
                                }
                                KeyCode::Tab => {
                                    app.switch_platform();
                                    if !app.search_query.is_empty() {
                                        app.trigger_search();
                                    }
                                }
                                KeyCode::Char('/') | KeyCode::Char('s') => {
                                    app.search_input.clear();
                                    app.mode = AppMode::SearchInput;
                                }
                                KeyCode::Char('c') => {
                                    if app.selected_repo().is_some() {
                                        app.prepare_clone_modal();
                                    }
                                }
                                KeyCode::Char('o') => {
                                    app.open_selected_repo_in_browser();
                                }
                                KeyCode::Char('r') => {
                                    if app.focused_pane == FocusedPane::RepoList {
                                        app.trigger_search();
                                    } else {
                                        app.trigger_load_current_folder();
                                    }
                                }
                                KeyCode::Char('j') | KeyCode::Down => match app.focused_pane {
                                    FocusedPane::RepoList => {
                                        app.select_next_repo();
                                    }
                                    FocusedPane::FileList => {
                                        app.select_next_file();
                                        app.trigger_load_preview();
                                    }
                                    FocusedPane::Preview => {
                                        app.preview_scroll = app.preview_scroll.saturating_add(1);
                                    }
                                },
                                KeyCode::Char('k') | KeyCode::Up => match app.focused_pane {
                                    FocusedPane::RepoList => {
                                        app.select_prev_repo();
                                    }
                                    FocusedPane::FileList => {
                                        app.select_prev_file();
                                        app.trigger_load_preview();
                                    }
                                    FocusedPane::Preview => {
                                        app.preview_scroll = app.preview_scroll.saturating_sub(1);
                                    }
                                },
                                KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                                    match app.focused_pane {
                                        FocusedPane::RepoList => {
                                            if app.selected_repo().is_some() {
                                                app.focused_pane = FocusedPane::FileList;
                                                app.current_path.clear();
                                                app.trigger_load_current_folder();
                                            }
                                        }
                                        FocusedPane::FileList => {
                                            if let Some(file) = app.selected_file() {
                                                if file.file_type == FileType::Directory {
                                                    let dir_name = file.name.clone();
                                                    app.current_path.push(dir_name);
                                                    app.trigger_load_current_folder();
                                                } else {
                                                    app.trigger_load_preview();
                                                    app.focused_pane = FocusedPane::Preview;
                                                }
                                            }
                                        }
                                        FocusedPane::Preview => {}
                                    }
                                }
                                KeyCode::Char('h') | KeyCode::Left => match app.focused_pane {
                                    FocusedPane::Preview => {
                                        app.focused_pane = FocusedPane::FileList;
                                    }
                                    FocusedPane::FileList => {
                                        if !app.current_path.is_empty() {
                                            app.current_path.pop();
                                            app.trigger_load_current_folder();
                                        } else {
                                            app.focused_pane = FocusedPane::RepoList;
                                            app.files.clear();
                                        }
                                    }
                                    FocusedPane::RepoList => {
                                        app.show_dashboard = true;
                                        app.status_message = "Press '/' to search, 'Tab' to switch platform, '?' for help".to_string();
                                    }
                                },
                                _ => {}
                            }
                        }
                    }

                    AppMode::SearchInput => match key.code {
                        KeyCode::Enter => {
                            if !app.search_input.is_empty() {
                                let q = app.search_input.value().trim().to_string();
                                if !q.is_empty() {
                                    app.search_query = q;
                                    app.mode = AppMode::Normal;
                                    app.trigger_search();
                                } else {
                                    app.mode = AppMode::Normal;
                                }
                            } else {
                                app.mode = AppMode::Normal;
                            }
                        }
                        KeyCode::Esc => {
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Left => {
                            app.search_input.move_left();
                        }
                        KeyCode::Right => {
                            app.search_input.move_right();
                        }
                        KeyCode::Home => {
                            app.search_input.move_home();
                        }
                        KeyCode::End => {
                            app.search_input.move_end();
                        }
                        KeyCode::Backspace => {
                            app.search_input.backspace();
                        }
                        KeyCode::Delete => {
                            app.search_input.delete();
                        }
                        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.search_input.move_home();
                        }
                        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.search_input.move_end();
                        }
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.search_input.clear();
                        }
                        KeyCode::Char(c) => {
                            app.search_input.insert_char(c);
                        }
                        _ => {}
                    },

                    AppMode::CloneModal => match key.code {
                        KeyCode::Esc => {
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Tab => {
                            app.clone_focused_field = (app.clone_focused_field + 1) % 3;
                        }
                        KeyCode::BackTab => {
                            app.clone_focused_field = if app.clone_focused_field == 0 {
                                2
                            } else {
                                app.clone_focused_field - 1
                            };
                        }
                        KeyCode::Enter => {
                            app.trigger_start_cloning();
                        }
                        // Clone method selector (0)
                        KeyCode::Left | KeyCode::Up if app.clone_focused_field == 0 => {
                            let methods = CloneMethod::all();
                            let current_idx = methods
                                .iter()
                                .position(|m| *m == app.clone_method)
                                .unwrap_or(0);
                            let new_idx = if current_idx == 0 {
                                methods.len() - 1
                            } else {
                                current_idx - 1
                            };
                            app.clone_method = methods[new_idx];
                        }
                        KeyCode::Right | KeyCode::Down if app.clone_focused_field == 0 => {
                            let methods = CloneMethod::all();
                            let current_idx = methods
                                .iter()
                                .position(|m| *m == app.clone_method)
                                .unwrap_or(0);
                            let new_idx = (current_idx + 1) % methods.len();
                            app.clone_method = methods[new_idx];
                        }
                        // Up/Down field switching
                        KeyCode::Up if app.clone_focused_field == 1 => {
                            app.clone_focused_field = 0;
                        }
                        KeyCode::Down if app.clone_focused_field == 1 => {
                            app.clone_focused_field = 2;
                        }
                        KeyCode::Up if app.clone_focused_field == 2 => {
                            app.clone_focused_field = 1;
                        }
                        KeyCode::Down if app.clone_focused_field == 2 => {
                            app.clone_focused_field = 0;
                        }
                        // Target directory input (1)
                        KeyCode::Left if app.clone_focused_field == 1 => {
                            app.clone_dir_input.move_left();
                        }
                        KeyCode::Right if app.clone_focused_field == 1 => {
                            app.clone_dir_input.move_right();
                        }
                        KeyCode::Home if app.clone_focused_field == 1 => {
                            app.clone_dir_input.move_home();
                        }
                        KeyCode::End if app.clone_focused_field == 1 => {
                            app.clone_dir_input.move_end();
                        }
                        KeyCode::Backspace if app.clone_focused_field == 1 => {
                            app.clone_dir_input.backspace();
                        }
                        KeyCode::Delete if app.clone_focused_field == 1 => {
                            app.clone_dir_input.delete();
                        }
                        KeyCode::Char('a')
                            if app.clone_focused_field == 1
                                && key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            app.clone_dir_input.move_home();
                        }
                        KeyCode::Char('e')
                            if app.clone_focused_field == 1
                                && key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            app.clone_dir_input.move_end();
                        }
                        KeyCode::Char('u')
                            if app.clone_focused_field == 1
                                && key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            app.clone_dir_input.clear();
                        }
                        KeyCode::Char(c) if app.clone_focused_field == 1 => {
                            app.clone_dir_input.insert_char(c);
                        }
                        // Branch input (2)
                        KeyCode::Left if app.clone_focused_field == 2 => {
                            app.clone_branch_input.move_left();
                        }
                        KeyCode::Right if app.clone_focused_field == 2 => {
                            app.clone_branch_input.move_right();
                        }
                        KeyCode::Home if app.clone_focused_field == 2 => {
                            app.clone_branch_input.move_home();
                        }
                        KeyCode::End if app.clone_focused_field == 2 => {
                            app.clone_branch_input.move_end();
                        }
                        KeyCode::Backspace if app.clone_focused_field == 2 => {
                            app.clone_branch_input.backspace();
                        }
                        KeyCode::Delete if app.clone_focused_field == 2 => {
                            app.clone_branch_input.delete();
                        }
                        KeyCode::Char('a')
                            if app.clone_focused_field == 2
                                && key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            app.clone_branch_input.move_home();
                        }
                        KeyCode::Char('e')
                            if app.clone_focused_field == 2
                                && key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            app.clone_branch_input.move_end();
                        }
                        KeyCode::Char('u')
                            if app.clone_focused_field == 2
                                && key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            app.clone_branch_input.clear();
                        }
                        KeyCode::Char(c) if app.clone_focused_field == 2 => {
                            app.clone_branch_input.insert_char(c);
                        }
                        _ => {}
                    },

                    AppMode::CloneFinished => match key.code {
                        KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                            app.mode = AppMode::Normal;
                        }
                        _ => {}
                    },

                    AppMode::HelpModal => match key.code {
                        KeyCode::Char('?') | KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                            app.mode = AppMode::Normal;
                        }
                        _ => {}
                    },

                    AppMode::CloningInProgress => {}
                }
            }
        }
    }
}
