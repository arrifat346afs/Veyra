# Veyra

A file manager for Linux, built in Rust with [GPUI](https://www.gpui.rs/) and
[gpui-kit](https://crates.io/crates/gpui-kit).

## Features

- Grid and list views with adjustable icon size, label size, and spacing
- Thumbnails: images and GIFs are decoded with `image-rs`; other formats preview
  through any freedesktop thumbnailer installed on the system
- Sidebar with home places and persistent bookmarks
- Breadcrumb navigation with back/forward history
- Live filtering of the current folder, with an optional case-sensitive mode
- Sorting by name, size, modified time, or type; folders-first option
- Multi-select with Ctrl/Shift click and a rubber-band drag
- Dark/light theme plus window, sidebar, and surface opacity sliders
- Searchable settings, persisted across launches

## Requirements

- Linux (X11 or Wayland) with a Vulkan-capable GPU; GPUI renders through Vulkan
  on Linux (software Vulkan such as lavapipe also works)
- Rust 1.85 or newer (the crate uses edition 2024)
- The [Papirus icon theme](https://github.com/PapirusDevelopmentTeam/papirus-icon-theme)
  (`papirus-icon-theme` on most distributions) for file and folder icons
- Optional: `ffmpegthumbnailer` or another freedesktop thumbnailer for video and
  document previews

## Build and run

```sh
git clone https://github.com/arrifat346afs/Veyra.git
cd Veyra
cargo run --release
```

The binary is written to `target/release/veyra`.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+H` | Toggle hidden files (dotfiles) |
| `Ctrl+,` | Open settings |
| `Escape` | Close settings |
| `Ctrl` / `Shift` + click | Add or toggle a file in the selection |
| Drag on empty space | Rubber-band select |

## Data locations

Veyra follows the XDG base directory specification:

| What | Path |
| --- | --- |
| Settings | `$XDG_CONFIG_HOME/veyra/settings.json` (default `~/.config/...`) |
| Bookmarks | `$XDG_DATA_HOME/filemanager/bookmarks` (default `~/.local/share/...`) |
| Thumbnail cache | `$XDG_CACHE_HOME/veyra/thumbnails/v1` (default `~/.cache/...`) |

On first run the bookmark file is seeded once from GTK's bookmark list, if present.
After that Veyra owns the list and never reads GTK's file again.

## Project layout

```
src/
  app.rs            application setup and theme registration
  windows/          main window state, navigation, and rendering
  layouts/          topbar, sidebar, and content pane
  settings/         preferences, settings panel, and reusable controls
  selection/        click, multi-select, and rubber-band selection
  thumbnails/       thumbnail cache and freedesktop thumbnailer discovery
  services/         persistent bookmarks
  keyboard_shortcuts/
```

## Status

Early development. Drive mounting, removable media, network shares, file
operations (copy/move/delete), and drag-and-drop are not implemented yet.
