# AutoDragFile

A fast, lightweight Rust utility that helps you programmatically initiate file drag-and-drop operations.

When executed, a small box spawns directly beneath your cursor. You have 3 seconds to click and start dragging the file. Once you drop the file (or cancel the drag), the program automatically exits. If you don't start dragging within the 3-second window, it safely times out and closes out of your way.

Works on **Wayland** (KDE Plasma / KWin and other compositors supporting `wlr-layer-shell`) and on **X11**.

### How it works on Wayland
Wayland doesn't let applications read the global cursor position or place their own windows. AutoDragFile therefore opens a transparent, fullscreen overlay using the layer-shell protocol, takes the cursor position from the first pointer event on that overlay, and then shrinks the overlay's input region to the box. Clicks outside the box go through to the windows underneath, and when a drag starts the input region is cleared so the drop reaches its target.

If layer-shell isn't available (e.g. on X11), a small borderless window is placed under the cursor instead.

## Usage
You can run it directly using Cargo:

```
cargo run --release -- /path/to/your/file
```

Alternatively, you can build the binary and run it directly:
```
# Build the executable
cargo build --release

# Run the compiled binary
./target/release/AutoDragFile /path/to/your/file
```

## Configuration
All texts, colours, sizes and timings can be changed with a TOML config file. Write the default config to a file, edit it, and pass it with `--config`:

```
AutoDragFile --print-default-config > ~/.config/autodragfile.toml
AutoDragFile --config ~/.config/autodragfile.toml /path/to/your/file
```

Every key is optional: anything you leave out falls back to its default, so a config can be as small as

```toml
[style]
background = "darkred"

[behavior]
timeout_ms = 5000
```

The default UI text is German ("📎 Datei ziehen").

## Dependencies
If you intend to only use the binary and not compile it yourself, you will only need `gtk3` and `gtk-layer-shell`!

This tool requires the Rust toolchain (installed via `rustup` or your package manager) plus the GTK 3 and gtk-layer-shell development headers to build.

> Note: `pkgconf` and `base-devel` are required to link the GTK libraries during the Rust build process.

#### Arch / Manjaro:
```
sudo pacman -S gtk3 gtk-layer-shell pkgconf
```

#### Ubuntu / Debian:
```
sudo apt install libgtk-3-dev libgtk-layer-shell-dev pkg-config build-essential
```
