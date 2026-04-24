# drmenu

[![tests](https://github.com/samflores/drmenu/actions/workflows/tests.yml/badge.svg)](https://github.com/samflores/drmenu/actions/workflows/tests.yml)

A GTK4 layer-shell menu launcher for Wayland. Reads entries from stdin,
shows them as a fuzzy-filterable horizontal list anchored to the bottom
of the screen, and prints the selected entry on activation.

## What it does

- Reads lines from stdin in the form `label,icon,value` (icon and value optional).
- Displays each line as a clickable item with optional icon.
- Filters the list as you type using fuzzy matching.
- On <kbd>Enter</kbd> (or <kbd>Ctrl</kbd>+<kbd>Y</kbd>), prints the selected
  entry's `value` (or `label` if no value is set) to stdout and exits.
- <kbd>Escape</kbd> exits without printing.

### Keybindings

| Key                                  | Action                |
| ------------------------------------ | --------------------- |
| <kbd>Tab</kbd> / <kbd>Ctrl</kbd>+<kbd>N</kbd>    | Select next item      |
| <kbd>Shift</kbd>+<kbd>Tab</kbd> / <kbd>Ctrl</kbd>+<kbd>P</kbd> | Select previous item |
| <kbd>Enter</kbd> / <kbd>Ctrl</kbd>+<kbd>Y</kbd>  | Activate selection    |
| <kbd>Escape</kbd>                    | Quit                  |

## Dependencies

Runtime / build system libraries:

- GTK 4 (`libgtk-4-dev` on Debian/Ubuntu, `gtk4` on Arch)
- `gtk4-layer-shell` ≥ 1.0 (Wayland layer-shell support)
- A Wayland compositor that implements the `wlr-layer-shell` protocol
  (Sway, Hyprland, River, Wayfire, etc.)

Build tools: Rust stable edition 2024, `pkg-config`, `meson`/`ninja`
(only if building `gtk4-layer-shell` from source).

## Build

```sh
cargo build --release
```

The resulting binary is `target/release/drmenu`.

## Usage

`drmenu` reads one entry per line from stdin. Each line has up to three
comma-separated fields:

```
label[,icon[,value]]
```

- `label` — required; shown in the UI.
- `icon`  — optional; absolute path to an image file, or empty.
- `value` — optional; printed on activation instead of `label`.

### Examples

A plain chooser piped into a command:

```sh
printf 'Firefox\nChromium\nThunderbird\n' | drmenu
```

With explicit values (e.g., commands to run):

```sh
cat <<'EOF' | drmenu | sh
Firefox,,firefox
Chromium,,chromium
Terminal,,foot
EOF
```

With icons from `.desktop` files:

```sh
cat <<'EOF' | drmenu
Firefox,/usr/share/icons/hicolor/48x48/apps/firefox.png,firefox
Thunderbird,/usr/share/icons/hicolor/48x48/apps/thunderbird.png,thunderbird
EOF
```

### Bind it to a key

In Sway (`~/.config/sway/config`):

```
bindsym $mod+d exec 'my-launcher-script | drmenu | sh'
```

## Tests

```sh
cargo test                       # unit tests
cargo test --features gtk-tests  # + GTK integration tests (needs a display)
```

See `.cargo/config.toml` for `cargo cov`, `cargo cov-html`,
`cargo cov-missing`, and `cargo mut` aliases.
