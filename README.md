# Veyra

A file manager for Linux, built in Rust with [GPUI](https://www.gpui.rs/) and
[gpui-kit](https://crates.io/crates/gpui-kit).

## Features

- Grid and list views with adjustable icon size, label size, and spacing
- Thumbnails: images and GIFs are decoded with `image-rs`; other formats preview
  through any freedesktop thumbnailer installed on the system
- File and folder creation, rename, copy, cut/move, paste, trash, and permanent delete
- Open files in their default application for reading and editing
- Open with a searchable popup of installed applications, with compatible apps first
- Veyra-owned Space-bar Quick Preview with image zoom and document page navigation;
  DOCX text fallback when LibreOffice is unavailable
- Sidebar with home places and persistent bookmarks
- Bookmark folders from their context menu or by dragging them onto the sidebar
- Breadcrumb navigation with back/forward history
- Live filtering of the current folder, with an optional case-sensitive mode
- Sorting by name, size, modified time, or type; folders-first option
- Multi-select with Ctrl/Shift click and a rubber-band drag
- Dark/light and custom JSON themes with live reload when the theme file changes
- Folder icons follow the theme accent while preserving their shading and symbols
- Window, sidebar, and surface opacity sliders
- Shared corner-radius slider for controls, menus, hover highlights, and drag selections
- Searchable settings in a separate fixed-size window, persisted across launches
- Desktop portal file picker for open/upload, folder selection, and save dialogs

## Requirements

- Linux (X11 or Wayland) with a Vulkan-capable GPU; GPUI renders through Vulkan
  on Linux (software Vulkan such as lavapipe also works)
- Rust 1.85 or newer (the crate uses edition 2024)
- The [Papirus icon theme](https://github.com/PapirusDevelopmentTeam/papirus-icon-theme)
  (`papirus-icon-theme` on most distributions) for file and folder icons
- GLib/GIO development libraries (`glib2` on Arch, `libglib2.0-dev` on Debian/Ubuntu)
  for installed application discovery and launching
- `gio` (GLib utilities) for desktop trash support
- Optional: Poppler (`poppler` on Arch, `poppler-utils` on Debian/Ubuntu) for PDF
  and formatted document previews
- Optional: LibreOffice (native or Flatpak) for formatted Word/Office previews;
  DOCX text previews also work without it
- Optional: `ffmpegthumbnailer` or another freedesktop thumbnailer for video and
  document previews

## Build and run

```sh
git clone https://github.com/arrifat346afs/Veyra.git
cd Veyra
cargo run --release
```

The binary is written to `target/release/veyra`.

### Build with GitHub Actions

Open **Actions → Build Linux → Run workflow** and select the branch to build.
The workflow runs for pull requests, version tags beginning with `v`, and
manual runs. Pushes to `main` do not start a build. The manual run button
becomes available once the workflow is on the repository's default branch.

When the run succeeds, download **veyra-linux-x86_64** from its **Artifacts**
section. The artifact contains a `.tar.gz` archive and an AppImage. Make the
AppImage executable and run it directly:

```sh
chmod +x Veyra-*-x86_64.AppImage
./Veyra-*-x86_64.AppImage
```

Or unzip the artifact and extract the `.tar.gz` archive to run the standalone
binary:

```sh
sha256sum -c veyra-linux-x86_64.tar.gz.sha256
tar -xzf veyra-linux-x86_64.tar.gz
cd veyra-linux-x86_64
./veyra
```

The `.tar.gz` file is a compressed application archive, not an Arch package;
`pacman -U` cannot install it. Extract it with `tar -xzf`, then run `./veyra`
or use `./scripts/install-desktop.sh` to add it to your user app menu. To
install through pacman, download the `.pkg.tar.zst` file from the release and
run `sudo pacman -U ./veyra-*.pkg.tar.zst`.

The archive includes the release binary and the portal integration scripts.
It targets x86_64 Linux and is built on Ubuntu 24.04; it requires compatible
system libraries and the runtime requirements above. It does not bundle those
libraries. Workflow artifacts are retained for 30 days.

To add Veyra to your desktop app menu with its icon, extract the archive and run
`./scripts/install-desktop.sh`. This installs the binary to `~/.local/bin` and
the launcher and icon to your user data directory. You can also run `./veyra`
directly from the extracted folder.

### Publish a GitHub Release

Pull requests build the app and upload a workflow artifact. Pushes to `main`
do not trigger builds. To publish a GitHub Release, manually run **Build Linux**
from the Actions page.
That manual run publishes a `v`-prefixed package version (for example,
`v0.1.0`), the application archive, an AppImage, and an Arch package
(`.pkg.tar.zst`), with SHA-256 checksums. The workflow reads the version from
`Cargo.toml`; update that version when you want to publish a new release.
Manual builds for the same version update that release's assets.

For a named version release, push a version tag instead:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The workflow uses the version tag as the release tag when building a tag.
Releases include generated release notes, `veyra-linux-x86_64.tar.gz`, an
AppImage, the Arch package, and their SHA-256 checksums. Download the AppImage
from the repository's **Releases** page, mark it executable, and run it. The
other assets are also available there. Release assets do not have the workflow
artifacts' 30-day expiry.

You can also run **Build Linux** manually against an existing `v*` tag to
publish it. Rerunning a workflow run replaces the matching assets on its
existing release.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+Shift+N` | Create a folder |
| `Ctrl+Shift+V` | Cycle grid, list, and column views |
| `Ctrl++` / `Ctrl+-` | Increase / decrease content icon size |
| `Ctrl+0` | Reset content icon size |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy / cut / paste |
| `Ctrl+Z` / `Ctrl+Y` / `Ctrl+Shift+Z` | Undo / redo |
| `Ctrl+A` | Select all visible files |
| `F2` | Rename the selected item |
| `Delete` | Move selected items to trash |
| `Shift+Delete` | Permanently delete selected items, with confirmation |
| Arrow keys | Move selection in the file grid or list |
| `Home` / `End` | Select the first / last visible item |
| `Backspace` | Go to the parent folder |
| `Alt+Left` / `Alt+Right` | Go back / forward in folder history |
| `Enter` | Open selected items |
| `Space` | Quick Preview of the selected file; Space/Escape closes the preview |
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

Early development. Drive mounting, removable media, network shares, and
drag-and-drop file moves are not implemented yet.

## Use Veyra as the system file picker

Veyra includes an `org.freedesktop.impl.portal.FileChooser` backend. Upload/open
requests from applications using XDG Desktop Portal open a Veyra picker, with
its usual sidebar, bookmarks, thumbnails, search, and grid/list views. The picker
returns selected file URIs to the requesting application. Veyra does not need to
be running beforehand.

Supports single/multiple files, folders, glob/MIME filters, application-provided
choices, save dialogs with replacement confirmation, and SaveFiles destination
selection. Click the file-type or choice button to cycle through its options.
Double-click a folder to browse it; double-click a file or use Open to select it.
Escape, Cancel, or closing the window cancels the request.

Open **Settings → General → Use Veyra for open and save dialogs** and turn on
the switch. Veyra installs its picker for your user account, keeps your previous
picker as a fallback, and offers **Restart now** to apply the change. No terminal
or administrator password is required. Restarting the portal closes current
portal dialogs and may interrupt screen sharing; choose **Later** to defer it.

The settings panel shows registration/startup status and offers **Refresh
status** and **Apply change…** when a restart is pending. Search settings for
“file picker”, “upload”, or “portal” to find it. Turn the switch off to restore
the previous picker configuration. Changes made outside Veyra are preserved.
If your portal list is managed by the system, the panel explains why the switch
is unavailable.

The settings installer stores a stable copy of the current Veyra executable in
`$XDG_DATA_HOME/veyra/portal/veyra`, registers it through per-user D-Bus activation,
and writes `veyra.portal` in `$XDG_DATA_HOME/xdg-desktop-portal/portals`. Recovery
state is kept in `$XDG_STATE_HOME/veyra/portal-setup.json`. Other portal preferences
are retained. A modern `xdg-desktop-portal` with per-user backend discovery is
required.

For distribution packaging, the optional `scripts/install-portal.sh` still
supports `PREFIX`, `PORTAL_DIR`, and `DESTDIR`. The settings flow does not depend
on these scripts or the source checkout.

Applications using their own file dialogs need to opt into desktop portals.
For GTK applications this can be requested when launching, for example:

```sh
GTK_USE_PORTAL=1 your-application
```

Support varies by application. The picker currently opens as an independent
window: attaching it as a modal/transient child of the requesting application
through X11/Wayland parent-window handles is not implemented.

Try the picker without changing desktop settings (prints JSON on acceptance;
prints nothing on cancellation):

```sh
cargo run -- --pick-file
cargo run -- --pick-file --multiple
cargo run -- --pick-file --directory
```

Portal integration follows the upstream [FileChooser backend interface](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.impl.portal.FileChooser.html)
and [backend configuration](https://flatpak.github.io/xdg-desktop-portal/docs/portals.conf.html).

## Quick Preview

Select one file and press **Space**, or choose **Quick preview** from its context
menu. Every preview opens in Veyra's own themed window. Space or Escape closes it;
text fields continue to accept spaces normally. Previews never change your files
or default applications.

Images (including SVG) support zoom and Fit controls. PDF pages render with
Poppler and have Previous/Next controls; Left/Right also change pages. Word and
other Office documents can be converted to page previews using headless
LibreOffice with an isolated temporary profile. All rendering happens in the
background, and generated files are removed when the preview closes.

DOCX/DOCM files fall back to a read-only text view if LibreOffice is unavailable
or conversion fails. That view includes paragraphs and table text but omits
formatting and embedded images. Plain text, code, and extensionless text files
also have a native text preview. Video files can show a still frame when FFmpeg
is installed; playback is available through Open file. Other file types show
file details or a clear missing-renderer message in the same preview window.
