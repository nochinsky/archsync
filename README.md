# archsync

[![CI](https://github.com/nochinsky/archsync/actions/workflows/ci.yml/badge.svg)](https://github.com/nochinsky/archsync/actions/workflows/ci.yml)

Save and restore your explicitly installed Arch Linux packages with a small Rust CLI.
Official repository packages and foreign packages are recorded separately in an editable TOML snapshot.

## Install

Build from source with Rust and Cargo:

```sh
git clone https://github.com/nochinsky/archsync.git
cd archsync
cargo build --release --locked
sudo install -m 755 target/release/archsync /usr/local/bin/archsync
```

Precompiled binaries, when available, are listed on the [Releases page](https://github.com/nochinsky/archsync/releases).

## Save a package list

Run this on your existing Arch system:

```sh
archsync save
```

This writes `archpkglist.toml` in the current directory. It records explicitly installed packages, separating official packages into `[repo]` and foreign packages into `[aur]`.

```toml
timestamp = "2026-10-12T00:00:00Z"

[repo]
git = true
neovim = true

[aur]
example-aur-package = false
```

Package names in the example are illustrative. Set an entry to `false` to skip it during restoration.
Keep a copy of the snapshot before running `save` again, which overwrites the current file.

## Restore a package list

Copy the snapshot to the target system, change to its directory, and run:

```sh
archsync restore
```

- Official packages are installed with `sudo pacman -S --needed`.
- Enabled AUR entries are installed with `yay -S --needed`, falling back to `paru` when yay is unavailable.
- An AUR helper is required only when an `[aur]` entry is `true`. An absent or empty AUR section, or one containing only disabled entries, needs no helper.
- A required helper is checked before package installation starts. Package manager prompts remain interactive.

Review foreign packages before restoring: packages reported by `pacman -Qm` are not necessarily available from the AUR.
The snapshot records package names and selection flags; it does not pin versions, back up configuration files, or remove extra packages from the target system.

## Requirements

Arch Linux, `pacman`, and `sudo` for official package installation. Install `yay` or `paru` separately when your snapshot needs AUR packages.

## Development

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

The restore regression tests run the real CLI with mocked package managers; they do not install packages on the test machine.

## License

[MIT](LICENSE).
