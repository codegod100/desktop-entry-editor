# Desktop Entry Editor

A Slint-based `.desktop` file editor, inspired by [desktop-file-search](https://github.com/codegod100/desktop-file-search).

Browse, search, and edit XDG desktop entries with a clean dark UI built with [Slint](https://slint.dev).

## Features

- **Search** installed `.desktop` files across standard XDG paths
- **Edit** all desktop entry fields: Name, Comment, Icon, Exec, Categories, MimeTypes, etc.
- **Flags** panel for toggling Terminal, NoDisplay, Hidden, StartupNotify, D-Bus activatable
- **Raw** editor tab for direct key-value editing
- **Create** new desktop entries in `~/.local/share/applications/`
- **Delete** entries with one click
- **Open** in your `$EDITOR` for advanced editing

## Building

```bash
nix build
./result/bin/desktop-entry-editor
```

## Development

```bash
nix develop
cargo run
```
