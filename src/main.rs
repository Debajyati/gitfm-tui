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
        terminal.draw(|f| ui::draw(f, app))?;

        // Poll events with a short timeout to allow async ticks and responsive renders
        if event::poll(Duration::from_millis(50))? {
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
                                        let _ = app.perform_search().await;
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
                                KeyCode::Char('r') => {
                                    if app.focused_pane == FocusedPane::RepoList {
                                        let _ = app.perform_search().await;
                                    } else {
                                        let _ = app.load_current_folder().await;
                                    }
                                }
                                KeyCode::Char('j') | KeyCode::Down => match app.focused_pane {
                                    FocusedPane::RepoList => {
                                        app.select_next_repo();
                                    }
                                    FocusedPane::FileList => {
                                        app.select_next_file();
                                        app.load_preview().await;
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
                                        app.load_preview().await;
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
                                                let _ = app.load_current_folder().await;
                                            }
                                        }
                                        FocusedPane::FileList => {
                                            if let Some(file) = app.selected_file() {
                                                if file.file_type == FileType::Directory {
                                                    let dir_name = file.name.clone();
                                                    app.current_path.push(dir_name);
                                                    let _ = app.load_current_folder().await;
                                                } else {
                                                    app.load_preview().await;
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
                                            let _ = app.load_current_folder().await;
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
                            let q = app.search_input.trim().to_string();
                            if !q.is_empty() {
                                app.search_query = q;
                                app.mode = AppMode::Normal;
                                let _ = app.perform_search().await;
                            } else {
                                app.mode = AppMode::Normal;
                            }
                        }
                        KeyCode::Esc => {
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Backspace => {
                            app.search_input.pop();
                        }
                        KeyCode::Char(c) => {
                            app.search_input.push(c);
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
                        KeyCode::Enter => {
                            app.start_cloning().await;
                        }
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
                        KeyCode::Backspace if app.clone_focused_field == 1 => {
                            app.clone_dir_input.pop();
                        }
                        KeyCode::Char(c) if app.clone_focused_field == 1 => {
                            app.clone_dir_input.push(c);
                        }
                        KeyCode::Backspace if app.clone_focused_field == 2 => {
                            app.clone_branch_input.pop();
                        }
                        KeyCode::Char(c) if app.clone_focused_field == 2 => {
                            app.clone_branch_input.push(c);
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
