# Veyra Feature Checklist

This checklist compares the requested GNOME Nautilus feature set with the code currently present in this repository. It describes Veyra's implementation, not features that might be supplied separately by the Linux desktop.

**Legend:** ✅ Implemented · 🟡 Partial or limited · ❌ Not implemented

## 1. File management

| Feature | Status | Notes |
|---|---|---|
| Create files and folders | ✅ | New folder and empty text document. |
| Copy, cut, paste, and move | ✅ | Clipboard file URI support; paste copies, cut/paste moves. |
| Rename files and folders | ✅ | Single-item rename. |
| Batch rename | ✅ | Replace and numbered/template modes, with preview. |
| Duplicate files | ✅ | Copy and paste in the same folder creates “name (copy)” and numbered copies without replacing the original. |
| Delete and restore files | ✅ | Move items to desktop Trash, restore them to their original locations, or permanently delete them. |
| Trash management | ✅ | Browse Trash, restore selected items, permanently delete selected items, or empty Trash. |
| Permanent deletion | ✅ | Confirmation before deletion. |
| Undo and redo | ✅ | Ctrl+Z / Ctrl+Y or Ctrl+Shift+Z and toolbar buttons reverse/replay create, rename, batch rename, links, copy/paste, moves, trash/restore, conflict replacements, archive creation and extraction. Permanent deletion and emptying Trash remain irreversible. |
| Create symbolic links | ✅ | Paste as link and a selected-item link wizard with a custom destination name. |
| Drag and drop | ✅ | Internal moves, native file-URI drops into the pane/folders/sidebar places, outbound single- or multi-selection file drags, and folder bookmarking are supported. External drops copy; internal drags move. |
| File operation progress | ✅ | Long-running operations report completed/total items and the current item name, including copy, move, trash, restore, delete, batch rename, and empty Trash. |
| File conflict handling | ✅ | Copy and move conflicts offer Replace, Skip, Keep Both, or Cancel. Same-folder copies automatically create numbered duplicates; replace keeps the previous item in Trash for undo. |

## 2. Navigation

| Feature | Status | Notes |
|---|---|---|
| Back and forward navigation | ✅ | Folder history with toolbar and keyboard shortcuts. |
| Parent folder navigation | ✅ | Backspace shortcut. |
| Breadcrumb navigation | ✅ | Clickable path segments. |
| Editable path bar | ✅ | Double-click the path; type a path and use folder suggestions. |
| Multiple tabs | ✅ | Ctrl+T opens a tab. With multiple tabs, a strip in the main pane above the toolbar lets you switch tabs, close any tab, or add one from the far right; the sidebar stays in place. Ctrl+Tab switches tabs, and Ctrl+W closes the active tab. Each tab restores its last folder. |
| Multiple windows | ✅ | The new-window control or Ctrl+N opens an independent file-browser window at the current folder. Settings and picker windows remain available. |
| Restore last location | ✅ | Reopens the last visited folder when starting the application. |
| Live directory updates | ✅ | Applies file operation results to the visible entries without resetting selection or scroll; the watcher picks up external changes and newly added items. |
| Keyboard shortcuts | ✅ | Navigation, selection, view, file operations, hidden files, and settings shortcuts are available. See the existing README. |
| Recent files | ✅ | Sidebar view records up to 100 files opened directly or with an application and keeps the list across launches. Missing files are omitted. |
| Starred files | ✅ | Toggle Starred from an item’s context menu; the sidebar view persists starred files and folders across launches. |
| Hidden files | ✅ | Toggle dotfiles with Ctrl+H or the context menu; preference is stored. |
| Context menus | ✅ | Actions follow the target: archive extraction appears for a supported archive, folder actions for a single folder, and batch actions for multiple selected items. File and folder menus include Copy for file transfer and Copy Path for copying the filesystem path as text. Empty-space and Trash menus provide directory-level actions. |

## 3. Sidebar and bookmarks

| Feature | Status | Notes |
|---|---|---|
| Home and standard folders | ✅ | Home, Desktop, Documents, Downloads, Pictures, Music, Videos, and filesystem root. |
| Custom bookmarks | ✅ | Persistent app-owned bookmarks with theme-accent folder icons; first run can import GTK bookmarks. |
| Add bookmarks | ✅ | Context menu or drag a folder to the sidebar. |
| Remove bookmarks | ✅ | Remove control appears when hovering over a bookmark row. |
| Open sidebar locations in a new tab or window | ✅ | Right-click a place or bookmark to open it in a new tab or window. |
| Rename bookmarks | ✅ | Rename from a bookmark's context menu; custom names persist with the bookmark. |
| Reorder places and bookmarks | ✅ | Drag any sidebar place or bookmark to a new position; order persists across launches. |
| Mounted drives | ✅ | GIO-mounted locations with local paths appear in the sidebar. |
| Removable devices | 🟡 | Mounted removable volumes that GIO marks ejectable appear; device eject/unmount controls are not implemented. |
| Network locations | 🟡 | Connected GIO network mounts with local paths appear; connecting to a server from the sidebar is not implemented. |
| Trash shortcut | ✅ | Sidebar entry opens the GIO Trash location. |

## 4. File views

| Feature | Status | Notes |
|---|---|---|
| Grid view | ✅ | Supported. |
| List view | ✅ | Supported. |
| Expandable folders in list view | ✅ | Click a folder row's disclosure chevron to show its children inline. Double-click still opens the folder. |
| Icon zoom controls | ✅ | Adjustable content icon size, including keyboard shortcuts. |
| File thumbnails | ✅ | Images/GIFs are decoded internally; SVG, video, PDF and Office documents have direct renderers, with registered desktop thumbnailers for additional formats. |
| Custom folder icons | ✅ | Choose an image in Veyra's file picker or reset the icon from a folder's context menu; choices persist per folder. |
| File type icons | ✅ | Uses Papirus icons selected from the system MIME database, with generic fallbacks. |
| File sorting | ✅ | Name, size, modified time, or type. |
| Reverse sorting | ✅ | Sort direction toggle. |
| Folders-first sorting | ✅ | Preference. |
| File metadata columns | ✅ | Right-click the list header to show or hide Size, Type, Modified, and Permissions columns. Choices are saved. |
| Selection information | ✅ | The bottom-right footer shows the selected item's name and size, or the selected item count and combined file size. |

## 5. Search

| Feature | Status | Notes |
|---|---|---|
| Filename search | ✅ | Live, case-configurable substring search; Ctrl+F focuses the search field. |
| Recursive folder search | ✅ | Entering a query searches the current folder and its descendants. |
| File type filters | ✅ | Search with `type:pdf` or a category such as `type:image`, `type:audio`, `type:video`, or `type:document`. |
| Date filters | ✅ | Search with `modified:7d`, `modified:24h`, or `modified:30m` to limit results by modification age. |
| Indexed content search | 🟡 | `content:word` scans common text files on demand (up to 1 MiB per file); no persistent index is built. |
| Recent file search | ✅ | A query filters the Recent and Starred lists without treating these virtual locations as directories. |
| Multiple search filters | ✅ | Combine a filename term and `type:`, `modified:`, and `content:` filters in one query. |

Search operators can be combined, for example `invoice type:pdf modified:30d`. Content search is limited to common text and source formats and reads matching files on demand.

## 6. File properties

| Feature | Status | Notes |
|---|---|---|
| File properties | ✅ | Properties dialog shows common filesystem details and embedded title, description, keywords, camera, image, document and media metadata when available through ExifTool. |
| File location | ✅ | Full path is displayed. |
| Creation/modification information | ✅ | Modified and created timestamps are displayed; unsupported creation time is shown as unavailable. |
| Folder size | ✅ | Directory contents are scanned in the background and file sizes are summed recursively. |
| Owner and group | ✅ | User and group names are shown, with numeric IDs when names cannot be resolved. |
| File permissions | ✅ | Unix mode bits are displayed and owner, group and other read/write/execute bits can be toggled. |
| Executable permission | ✅ | Executability is displayed and controlled through the execute permission toggles. |
| Default application selection | ✅ | Set Default Application or the Open with “Always use for this file type” switch updates the system MIME association. |
| Filesystem information | ✅ | Filesystem type appears in Properties; the File system sidebar entry shows a used-space percentage and progress bar. |

## 7. Archives

| Feature | Status | Notes |
|---|---|---|
| Compress files and folders | ✅ | Select files or folders, then choose Compress… from the context menu. |
| Create ZIP archives | ✅ | Creates recursive ZIP archives with file and folder structure preserved. |
| Create TAR archives | ✅ | Creates uncompressed TAR archives with file and folder structure preserved. |
| Extract archives | ✅ | Extracts ZIP and TAR variants directly; common additional formats use installed 7z or bsdtar backends. |
| Select extraction destination | ✅ | Extract to… and opening an archive prompt for a new destination-folder path; existing folders are never overwritten. |
| Additional archive formats through backends | ✅ | Supports common 7z, RAR, CAB, ISO, CPIO, AR, DEB, RPM, JAR, and compressed formats when installed 7z or bsdtar supports them. |

## 8. Network

| Feature | Status | Notes |
|---|---|---|
| SMB shares | ❌ | No Veyra network-share browser/mount integration. |
| SFTP connections | ❌ | Not implemented. |
| FTP connections | ❌ | Not implemented. |
| WebDAV | ❌ | Not implemented. |
| Remote filesystem browsing | ❌ | Browser navigation is based on local filesystem paths. |
| Network mounts | ❌ | No mount management. |
| Online accounts integration | ❌ | Not implemented. |
| Credential storage integration | ❌ | Not implemented. |

## 9. Devices

| Feature | Status | Notes |
|---|---|---|
| USB drive support | ❌ | No device discovery or management. |
| External HDD/SSD support | ❌ | No device discovery or management. |
| Partition browsing | 🟡 | Filesystem root can be browsed if already mounted and accessible; no partition/device list. |
| Mount and unmount | ❌ | Not implemented. |
| Safe eject | ❌ | Not implemented. |
| Android MTP support | ❌ | Not implemented. |
| Camera support | ❌ | Not implemented. |
| Storage information | ❌ | No device capacity/free-space view. |

## 10. Preview

| Feature | Status | Notes |
|---|---|---|
| Image thumbnails | ✅ | Built-in raster/GIF decoding and direct SVG rendering. |
| Video thumbnails | ✅ | Generates a video frame directly with FFmpeg; installed desktop thumbnailers remain a fallback. |
| Document thumbnails | ✅ | Generates the first PDF page with Poppler and Office document pages with LibreOffice; installed desktop thumbnailers handle additional formats. |
| Thumbnail caching | ✅ | Veyra cache under the XDG cache directory. |
| Quick preview with GNOME Sushi | ✅ | Space invokes GNOME Sushi through its D-Bus preview interface when available; Veyra's built-in preview is the fallback. |
| Image preview | ✅ | Veyra Quick Preview, with zoom. |
| PDF preview | ✅ | Multi-page preview uses Poppler when installed. |
| Text preview | ✅ | Read-only, bounded text preview. |
| Audio preview | ✅ | Space-bar preview identifies audio files and offers playback in the system's associated media application. |
| Video preview | ✅ | Shows a frame preview when FFmpeg is installed and offers playback in the associated media application. |

## 11. Customization

| Feature | Status | Notes |
|---|---|---|
| Default view preferences | ✅ | Grid/list/column choice is persisted; the selected choice uses the primary button style. |
| Icon sizing | ✅ | Adjustable and persisted. |
| Interface density | ✅ | Spacious, Comfortable, and Compact choices in Appearance settings. |
| Sorting preferences | ✅ | Sort key, direction, and folders-first behavior are supported. |
| Thumbnail preferences | ✅ | Thumbnail display can be toggled. |
| Hidden-file visibility | ✅ | Toggle is available and persisted. |
| Custom folder icons | ✅ | Choose or reset an image icon per folder from its context menu and Veyra's file picker. |
| System light/dark theme | 🟡 | Built-in light/dark choices and custom JSON themes; automatic system-theme following is not evident. |
| Keyboard navigation | ✅ | Selection and navigation support keyboard controls. |
| Accessibility support | 🟡 | Some controls have labels and keyboard handling; a complete accessibility audit/support layer is not evident. |
| Localization | ❌ | No translation/localization framework or translated strings found. |

## 12. Extensions

| Feature | Status | Notes |
|---|---|---|
| Nautilus scripts | 🟡 | Executable scripts in `~/.local/share/nautilus/scripts` appear in a Scripts submenu; nested script folders are not yet grouped into nested menus. |
| Extension API | ❌ | No Veyra extension API. |
| Custom context-menu actions | ✅ | Executable user scripts in `~/.local/share/nautilus/scripts` appear in a Scripts submenu and receive selected paths as arguments plus Nautilus-compatible environment variables. |
| Open in Terminal integration | ✅ | Context menu launches the available terminal command in the selected folder. |
| Third-party Git integrations | ❌ | No Git status/actions integration. |
| Application integrations | 🟡 | Opens files with system applications; Open with can optionally save the chosen app as the default handler for a single file's type. Includes an XDG Desktop Portal file chooser backend; no general application extension system. |

## How this was assessed

The checklist was assessed from the repository implementation and its existing README. A feature marked partial may depend on desktop utilities or providers installed on the host. “Not implemented” means no corresponding Veyra functionality was found in this codebase.

## Requested capability checklist

This section tracks the requested capability list. Status reflects Veyra itself; a desktop service or external application may provide related behavior without making it an implemented Veyra feature.

**Legend:** ✅ Implemented · 🟡 Partial or limited · ❌ Not implemented

### High priority

| Feature | Status | Notes |
|---|---|---|
| Connect to Server dialog for SMB, SFTP, and WebDAV | ❌ | No server connection dialog. Existing GIO network mounts may appear in the sidebar. |
| USB mount, unmount, and eject controls | ❌ | Mounted removable volumes may be listed; Veyra has no mount, unmount, or eject controls. |
| Drive and partition discovery | ❌ | Veyra does not discover or list devices and partitions. |
| Storage capacity and free-space display per device | 🟡 | The filesystem sidebar entry shows used-space percentage; it does not list capacity/free space per device. |
| Android MTP device browsing | ❌ | Not implemented. |
| Remote file copy, move, and deletion | ❌ | File operations use local filesystem paths; remote filesystem operations are not implemented. |

### Medium priority

| Feature | Status | Notes |
|---|---|---|
| Embedded terminal panel | ❌ | A context-menu action can open an external terminal in a folder. |
| Dual-pane / split file browsing | ❌ | Tabs and a column view exist; there are no two independently browsable panes. |
| Git branch and modification status indicators | ❌ | Not implemented. |
| Persistent full-text search index | 🟡 | Content search scans supported text files on demand; no persistent index is built. |
| Quick file-type and size filtering controls | 🟡 | Search supports file-type filters; no size filter controls were found. |
| Folder comparison and synchronization | ❌ | Not implemented. |
| Plugin and extension system | ❌ | No Veyra extension API or plugin loader. |
| Keyboard shortcut customization | ❌ | Shortcuts are built in; no customization interface was found. |
| Audio/video playback inside Quick Preview | 🟡 | Audio is identified and video can show a still frame; playback opens the system's associated media application. |
| Localization / translation support | ❌ | No translation framework or translated strings were found. |

### Optional

| Feature | Status | Notes |
|---|---|---|
| Cloud account integration | ❌ | Not implemented. |
| File tagging and color labels | ❌ | Starred items are supported, but tags and color labels are not. |
| Duplicate file finder | ❌ | Same-folder copy can create a duplicate; there is no duplicate detection/finder. |
| Checksum calculation and verification | ❌ | Not implemented in the app. |
| Visual disk usage analyzer | 🟡 | The filesystem sidebar shows used-space percentage, but there is no visual analyzer of folder or disk contents. |
| Saved searches | ❌ | Not implemented. |
| Per-folder custom view configuration | ❌ | View preferences are not stored per folder. |
| Session restoration for all tabs and windows | ❌ | The last location is restored, but the full tab/window session is not. |
| Automatic system theme detection | 🟡 | Light/dark choices and custom themes exist; automatic system-theme following is not implemented. |
| Nested Nautilus script submenus | 🟡 | Scripts are listed in a submenu; nested script directories are not shown as nested menus. |
