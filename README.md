# Veyra

A file manager for Linux, built in Rust with [GPUI](https://www.gpui.rs/) and
[gpui-kit](https://crates.io/crates/gpui-kit).

## Features

- Grid and list views with adjustable icon size, label size, and spacing
- Thumbnails: images and GIFs are decoded with `image-rs`; other formats preview
  through any freedesktop thumbnailer installed on the system
- File and folder creation, rename, copy, cut/move, paste, trash, and permanent delete
- Open files in their default application for reading and editing
- Sidebar with home places and persistent bookmarks
- Breadcrumb navigation with back/forward history
- Live filtering of the current folder, with an optional case-sensitive mode
- Sorting by name, size, modified time, or type; folders-first option
- Multi-select with Ctrl/Shift click and a rubber-band drag
- Dark/light and custom JSON themes with live reload when the theme file changes
- Folder icons follow the theme accent while preserving their shading and symbols
- Window, sidebar, and surface opacity sliders
- Searchable settings, persisted across launches

## Requirements

- Linux (X11 or Wayland) with a Vulkan-capable GPU; GPUI renders through Vulkan
  on Linux (software Vulkan such as lavapipe also works)
- Rust 1.85 or newer (the crate uses edition 2024)
- The [Papirus icon theme](https://github.com/PapirusDevelopmentTeam/papirus-icon-theme)
  (`papirus-icon-theme` on most distributions) for file and folder icons
- `gio` (GLib utilities) for desktop trash support
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
| `Ctrl+Shift+N` | Create a folder |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy / cut / paste |
| `Ctrl+A` | Select all visible files |
| `F2` | Rename the selected item |
| `Delete` | Move selected items to trash |
| `Shift+Delete` | Permanently delete selected items, with confirmation |
| `Enter` | Open selected items |
| `F5` / `Ctrl+R` | Reload the current directory |
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
  context_menu/     menu layout, commands, dialogs, clipboard, and file operations
  thumbnails/       thumbnail cache and freedesktop thumbnailer discovery
  services/         persistent bookmarks
  keyboard_shortcuts/
```

## Status

Early development. Drive mounting, removable media, network shares, file
operations (copy/move/delete), and drag-and-drop are not implemented yet.
