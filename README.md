# Markdown Todo

A Plasma 6 system tray widget. Dark Markdown popup, immediate checkbox saves, and an external editor. The popup closes when focus moves away.

## Build

Requires Rust stable, a C linker, Plasma 6, and Plasma5Support's executable data engine. Terminal editors also need a terminal that accepts `-e`. No Python runtime is needed.

On Fedora, install build tools if needed:

```sh
sudo dnf install cargo gcc
```

Build and test:

```sh
cargo test --locked
sh scripts/build.sh
```

The Rust helper lives inside the widget package. QML calls it through Plasma5Support. It runs only for file operations and editor launch. While the popup is open, it checks for external edits every two seconds.

## Configure

Create `~/.config/markdown-todo/config.json`, or use `$XDG_CONFIG_HOME/markdown-todo/config.json`:

```json
{
  "path": "~/todo.md",
  "editor": "/usr/bin/nvim",
  "is_graphical": false,
  "terminal": "/usr/bin/ghostty"
}
```

Use an absolute path or `~/`. Without a config file, the widget uses `~/todo.md`. Create the Markdown file before opening the widget. `examples/todo.md` is a starting point.

Set `is_graphical` to `true` to open the editor directly, for example `"editor": "kate"`. It defaults to `true`. With `false`, the terminal receives `-e`, the editor command, and the file path. `terminal` is an executable path or name, defaulting to `konsole`. Konsole also receives `--separate`.

If `editor` is absent, empty, or fails to launch, the widget tries `$VISUAL` directly as a graphical editor. Set it in the Plasma session environment, for example through `~/.config/environment.d/markdown-todo.conf`:

```ini
VISUAL=kate
```

Log out and in after changing the session environment. Config changes apply on the next action. Editor commands accept quoted arguments; shell expressions are not evaluated. Launch failures and nonzero exits within the first 200 ms trigger fallback. Errors after the editor has started cannot be detected. The editor controls saving.

## Install

After building, run:

```sh
kpackagetool6 --type Plasma/Applet --install package
```

Open **System Tray Settings**, enable **Markdown Todo** under **Entries**, and select **Always shown**. Plasma controls tray visibility; the widget always reports itself active. It creates no application window or taskbar entry.

To update:

```sh
sh scripts/build.sh
kpackagetool6 --type Plasma/Applet --upgrade package
```

Log out and in if Plasma still uses a cached version. To remove:

```sh
kpackagetool6 --type Plasma/Applet --remove io.github.mnjm.markdown-todo
```

## Behavior and checks

Headings, emphasis, links, lists, nested tasks, and fenced code render. Click a checkbox to save it. Task order and other checkboxes stay unchanged. Images and raw HTML are inert. Links open only HTTP, HTTPS, or mail URLs.

Stale checkbox clicks reload the file and ask you to click again. Saves use a temporary file and atomic replacement, preserving file permissions and symlinks. External editors do not share a lock with the widget; a simultaneous save in the brief gap between the final check and replacement can still race.

After installation, run the acceptance checks in [PRD.md](PRD.md), especially focus loss, a light desktop theme, editor launch, and an unwritable file. Automated tests cover Markdown rendering, exact checkbox changes, stale edits, and failed writes. They cannot verify the desktop's tray behavior.
