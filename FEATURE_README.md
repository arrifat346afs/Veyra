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
| Undo and redo | ✅ | Ctrl+Z / Ctrl+Y or Ctrl+Shift+Z and toolbar buttons reverse/replay create, rename, batch rename, links, copy/paste, moves, trash/restore, conflict replacements, and ZIP extraction. Permanent deletion and emptying Trash remain irreversible. |
| Create symbolic links | ✅ | Paste as link and a selected-item link wizard with a custom destination name. |
| Drag and drop | ✅ | Internal moves, native file-URI drops into the pane/folders/sidebar places, outbound file drags, and folder bookmarking are supported. External drops copy; internal drags move. |
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
| Live directory updates | ✅ | Refreshes the visible folder when files are created, renamed, moved, or removed externally. |
| Keyboard shortcuts | ✅ | Navigation, selection, view, file operations, hidden files, and settings shortcuts are available. See the existing README. |
| Recent files | ✅ | Sidebar view records up to 100 files opened directly or with an application and keeps the list across launches. Missing files are omitted. |
| Starred files | ✅ | Toggle Starred from an item’s context menu; the sidebar view persists starred files and folders across launches. |
| Hidden files | ✅ | Toggle dotfiles with Ctrl+H or the context menu; preference is stored. |
| Context menus | ✅ | Actions follow the target: archive extraction appears for a single ZIP, folder actions for a single folder, and batch actions for multiple selected items. Empty-space and Trash menus provide directory-level actions. |

## 3. Sidebar and bookmarks

| Feature | Status | Notes |
|---|---|---|
| Home and standard folders | ✅ | Home, Desktop, Documents, Downloads, Pictures, Music, Videos, and filesystem root. |
| Custom bookmarks | ✅ | Persistent app-owned bookmarks; first run can import GTK bookmarks. |
| Add bookmarks | ✅ | Context menu or drag a folder to the sidebar. |
| Remove bookmarks | ✅ | Remove control on each bookmark row. |
| Open sidebar locations in a new tab or window | ✅ | Right-click a place or bookmark to open it in a new tab or window. |
| Rename bookmarks | ❌ | Names are derived from the folder name. |
| Reorder bookmarks | ❌ | No reorder interaction or ordering controls. |
| Mounted drives | ❌ | No device discovery or drive list. |
| Removable devices | ❌ | No removable-device management. |
| Network locations | ❌ | No network-location browser. |
| Trash shortcut | ✅ | Sidebar entry opens the GIO Trash location. |

## 4. File views

| Feature | Status | Notes |
|---|---|---|
| Grid view | ✅ | Supported. |
| List view | ✅ | Supported. |
| Expandable folders in list view | ❌ | List rows do not provide tree expansion. |
| Icon zoom controls | ✅ | Adjustable content icon size, including keyboard shortcuts. |
| File thumbnails | 🟡 | Image/GIF handling and installed freedesktop thumbnailers; availability depends on installed providers and settings. |
| Custom folder icons | 🟡 | Folder icons are themed by Veyra; assigning arbitrary per-folder icons is not implemented. |
| File type icons | ✅ | Uses Papirus icons selected from the system MIME database, with generic fallbacks. |
| File sorting | ✅ | Name, size, modified time, or type. |
| Reverse sorting | ✅ | Sort direction toggle. |
| Folders-first sorting | ✅ | Preference. |
| File metadata columns | ❌ | List view does not offer configurable metadata columns. |
| Selection information | 🟡 | Selection is shown/highlighted; no selected-item details summary. The footer reports visible item count. |

## 5. Search

| Feature | Status | Notes |
|---|---|---|
| Filename search | 🟡 | Live substring filtering of the current directory only. |
| Recursive folder search | ❌ | Search does not traverse subfolders. |
| File type filters | 🟡 | Portal file-picker requests can filter by MIME type or glob; the normal browser search has no type-filter controls. |
| Date filters | ❌ | No date-based search filters. |
| Indexed content search | ❌ | No index or content search. |
| Recent file search | ✅ | The filename search filters the Recent view. |
| Multiple search filters | ❌ | Normal search supports a filename query and case-sensitivity preference only. |

## 6. File properties

| Feature | Status | Notes |
|---|---|---|
| File size and type | ✅ | Properties dialog reports type and size in bytes. |
| File location | ✅ | Full path is displayed. |
| Creation/modification information | ❌ | Timestamps are not displayed. |
| Folder size | 🟡 | Folder metadata length is shown, not a recursive total of its contents. |
| Owner and group | ❌ | Not displayed. |
| File permissions | 🟡 | Only a read-only yes/no value; no permission editor or detailed mode bits. |
| Executable permission | ❌ | No executable-bit control. |
| Default application selection | 🟡 | Files can be opened with their system default or a chooser for installed applications; no association-setting control is evident. |
| Filesystem information | ❌ | No filesystem/device properties panel. |

## 7. Archives

| Feature | Status | Notes |
|---|---|---|
| Compress files and folders | ❌ | No archive-creation command. |
| Create ZIP archives | ❌ | No ZIP creation. |
| Create TAR archives | ❌ | No TAR creation. |
| Extract archives | 🟡 | ZIP extraction only, to a new sibling folder. Opening a ZIP also extracts there. |
| Select extraction destination | ❌ | Destination is automatically derived from the archive name. |
| Additional archive formats through backends | ❌ | No general archive backend integration; other archive types are not extracted by Veyra. |

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
| Image thumbnails | ✅ | Built-in image decoding, including GIF handling. |
| Video thumbnails | 🟡 | Uses an installed freedesktop thumbnailer; depends on a suitable provider. |
| Document thumbnails | 🟡 | Uses installed thumbnailers where available. |
| Thumbnail caching | ✅ | Veyra cache under the XDG cache directory. |
| Quick preview with GNOME Sushi | ❌ | Veyra has its own Space-bar preview; it does not integrate GNOME Sushi. |
| Image preview | ✅ | Veyra Quick Preview, with zoom. |
| PDF preview | 🟡 | Requires supported installed renderers such as Poppler. |
| Text preview | ✅ | Read-only, bounded text preview. |
| Audio preview | ❌ | No audio playback/preview. |
| Video preview | 🟡 | Attempts a frame preview through available tools; depends on installed support. |

## 11. Customization

| Feature | Status | Notes |
|---|---|---|
| Default view preferences | ✅ | Grid/list/column choice is persisted; the selected choice uses the primary button style. |
| Icon sizing | ✅ | Adjustable and persisted. |
| Interface density | ✅ | Spacious, Comfortable, and Compact choices in Appearance settings. |
| Sorting preferences | ✅ | Sort key, direction, and folders-first behavior are supported. |
| Thumbnail preferences | ✅ | Thumbnail display can be toggled. |
| Hidden-file visibility | ✅ | Toggle is available and persisted. |
| Custom folder icons | 🟡 | The app styles folder icons; choosing an arbitrary icon per folder is not supported. |
| System light/dark theme | 🟡 | Built-in light/dark choices and custom JSON themes; automatic system-theme following is not evident. |
| Keyboard navigation | ✅ | Selection and navigation support keyboard controls. |
| Accessibility support | 🟡 | Some controls have labels and keyboard handling; a complete accessibility audit/support layer is not evident. |
| Localization | ❌ | No translation/localization framework or translated strings found. |

## 12. Extensions

| Feature | Status | Notes |
|---|---|---|
| Nautilus scripts | ❌ | No Nautilus script integration. |
| Extension API | ❌ | No Veyra extension API. |
| Custom context-menu actions | ❌ | Context menu actions are built into the application; no user/plugin action registration. |
| Open in Terminal integration | ✅ | Context menu launches the available terminal command in the selected folder. |
| Third-party Git integrations | ❌ | No Git status/actions integration. |
| Application integrations | 🟡 | Opens files with system applications and includes an XDG Desktop Portal file chooser backend; no general application extension system. |

## How this was assessed

The checklist was assessed from the repository implementation and its existing README. A feature marked partial may depend on desktop utilities or providers installed on the host. “Not implemented” means no corresponding Veyra functionality was found in this codebase.
