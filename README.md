# gitFM TUI 🦀

[![Rust](https://img.shields.io/badge/rust-1.90.0%2B-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Ratatui](https://img.shields.io/badge/Ratatui-0.29-red.svg?style=flat-square)](https://ratatui.rs/)
[![License: ISC](https://img.shields.io/badge/License-ISC-blue.svg?style=flat-square)](https://opensource.org/licenses/ISC)

> A blazing-fast, high-performance **Terminal User Interface (TUI)** for exploring and cloning GitHub & GitLab repositories. Features a **Yazi-inspired 3-column Miller layout**, interactive folder hierarchy navigation, instant file previews, and advanced Git cloning strategies (**Sparse-checkout**, **Shallow**, **Blobless**, **Treeless**, and **Normal**).

---

## ⚡ Highlights & Features

- 🗂️ **Yazi / Ranger-Style Miller Columns**:
  - **Left Pane**: Repository search results with star counts, language tags, and platform indicators.
  - **Middle Pane**: Interactive directory tree explorer (`📁` folders, `📄` files, sizes, breadcrumb navigation).
  - **Right Pane**: Rich preview pane showing repository details, directory actions, or raw file content.
- 🌳 **Drill Down & Sparse Checkout**:
  - Browse deep into remote monorepo hierarchies (`l` / `Enter` to step into folders, `h` to step out).
  - Press `c` on any subfolder to **Sparse Clone only that specific folder**!
- 🐙 **Octocrab & Rustls Powered**:
  - Native asynchronous GitHub API integration via `octocrab`.
  - Pure-Rust TLS with `rustls` (zero dependency on system OpenSSL libraries).
- 🦊 **GitLab Integration**:
  - Search public & internal GitLab projects, explore tree items, and clone seamlessly.
- 🔐 **Zero-Config Token Detection**:
  - Automatically detects and reads existing tokens from `~/.gitfmrc.json` (GitHub) and `~/.gl.gitfmrc.json` (GitLab) or `GITHUB_TOKEN` / `GITLAB_TOKEN` environment variables.
- ⚡ **Asynchronous Git Execution**:
  - Clones run asynchronously in the background with real-time feedback modals.

---

## ⌨️ Keybindings

### Navigation (Yazi / Vim-Style)

| Key | Action |
| :--- | :--- |
| `j` / `Down` | Move selection down |
| `k` / `Up` | Move selection up |
| `l` / `Right` / `Enter` | Enter folder / explore repository / view file |
| `h` / `Left` | Go up to parent folder / back to repository list |
| `PageDown` / `PageUp` | Scroll file preview |

### Actions & Cloning

| Key | Action |
| :--- | :--- |
| `c` | Open **Clone Dialog** (auto-targets current folder for Sparse Clone) |
| `/` or `s` | Open Search Prompt (query GitHub or GitLab) |
| `Tab` | Switch platform between **GitHub** and **GitLab** |
| `r` | Refresh current folder or repository search |
| `?` | Toggle Help & Keybindings Modal |
| `q` / `Esc` | Back / Close modal / Exit |
| `Ctrl+C` | Force Exit |

---

## 🚀 Cloning Modes

When you press `c`, the **Clone Dialog** lets you select the optimal cloning strategy:

1. **Normal Clone**: Standard complete clone with full commit history.
2. **Shallow Clone (`--depth 1`)**: Fetches only the latest commit. Minimal bandwidth.
3. **Blobless Clone (`--filter=blob:none`)**: Downloads commit and tree history; file blobs are fetched on-demand.
4. **Treeless Clone (`--filter=tree:0`)**: Downloads commit history only; trees and blobs are fetched on-demand.
5. **Sparse Checkout**: Clones **only** the subfolder you navigated into in the Miller column!

---

## 📦 Building & Running

### Prerequisites

- [Rust & Cargo](https://www.rust-lang.org/tools/install) (1.80+)
- `git` installed on your system

### Clone the Repo

```bash
git clone https://github.com/Debajyati/gitfm-tui
```

### Quick Run

```bash
cd gitfm-tui
cargo run --release
```

### Build Binary

```bash
cargo build --release
# The compiled executable is at: ./target/release/gitfm-tui
```

You can alias or copy it to your `PATH`:

```bash
cp target/release/gitfm-tui ~/.local/bin/gitfm-tui
```

---

## 🏛️ Project Architecture

```
gitfm-tui/
├── Cargo.toml          # Rust dependencies (ratatui, crossterm, octocrab, rustls, tokio)
└── src/
    ├── main.rs         # Terminal initialization, crossterm event loop, lifecycle
    ├── app.rs          # Central application state machine and navigation logic
    ├── ui.rs           # Ratatui rendering: Miller columns, modals, status bars
    ├── types.rs        # Data structures (RepoItem, FileItem, CloneMethod, Platform)
    ├── github.rs       # GitHub client using Octocrab
    ├── gitlab.rs       # GitLab v4 API client using Reqwest
    ├── git.rs          # Async Git subprocess runner (normal, partial, sparse)
    └── config.rs       # Automatic token discovery from gitFM dotfiles and environment
```

---

## 📄 License

Licensed under the [ISC License](https://opensource.org/licenses/ISC).
