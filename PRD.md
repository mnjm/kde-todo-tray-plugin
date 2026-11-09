# KDE Markdown todo

## Goal

Check tasks from the KDE system tray. Keep todos in a plain Markdown file and edit them with your preferred editor.

## Behavior

- Native Plasma 6 widget with an always-visible system tray icon.
- Rust handles Markdown and file operations; QML provides the Plasma interface.
- Clicking the icon opens a compact, lightweight, dark popup.
- Switching focus away hides the popup. It has no taskbar entry.
- The popup renders Markdown and accepts task checkbox clicks.
- Checking or unchecking saves immediately. Task order and child checkbox states stay unchanged.
- Completed task text is struck through. The popup uses gray accents and icon buttons for Edit and Reload.
- Edit uses the configured editor. `is_graphical: false` wraps it in the configured terminal. Missing or failed editor launches fall back to `$VISUAL` directly. The editor controls saving.

## File and rendering

- One local Markdown file is the source of truth. No accounts or sync.
- Configure its path in a file under `$XDG_CONFIG_HOME`, defaulting to `~/.config`. No settings screen.
- Support headings, bold/italic, links, lists, nested tasks, and fenced code blocks.
- Reload external file changes. Before saving a checkbox change, detect stale content and reload instead of overwriting it.
- Show read, write, or editor-launch errors in the popup. Never report an unsuccessful save as saved.

## Out of scope

Task reordering, embedded text editing, quick-add, multiple files, tables, images, raw HTML, automatic merging, and a theme switcher.

## Acceptance checks

- Open the popup, switch to another app, and confirm only the tray icon remains.
- Check and uncheck a nested task. Reopen the popup and file; only that checkbox changed.
- Save an external edit and confirm the popup reloads it. A stale checkbox click cannot overwrite it.
- Use Edit and confirm graphical, terminal, and `$VISUAL` fallback launches open the correct file.
- Confirm the popup stays dark under a light KDE theme and renders the supported Markdown.
- Make the file unwritable and confirm a checkbox click reports the save failure.

## Delivery

Working source and repeatable installation instructions for Plasma 6. The user installs it; development does not modify the active KDE session.
