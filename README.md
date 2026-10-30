# Fill-a-Pix puzzle studio

A desktop and command-line studio for designing, solving and printing Fill-a-Pix puzzles from images.

To work on it, run `nix develop` to enter the dev shell, then `cargo test` or `cargo run -p fap-cli -- --help`. Every cargo command needs the dev shell, since it provides the Rust toolchain.

Crate map: `fap-core` (grid, region and puzzle types, file formats) and `fap-cli` (command-line front end) exist now; `fap-solver`, `fap-pdf` and `fap-gui` will be added later, so this map will grow.
