# ziptree

Display the contents of an archive as a directory tree, without unpacking it (like [tree](https://oldmanprogrammer.net/source.php?dir=projects/tree)).

```
Three Seminars wrt pattern filler.zip
└── Three Seminars wrt pattern filler
    ├── 20201005 October 1 of 3
    │   ├── class patches
    │   │   ├── envelope 2.maxpat
    │   │   └── envelope.maxpat
    │   └── video recordings
    │       └── zoom_0.mp4
    ├── 20201012 October 2 of 3
    │   ├── chat.txt
    │   └── zoom_0.mp4
    └── 20201019 October 3 of 3
        └── recording.mp4

8 directories, 12 files
```

## Supported formats

`.zip`, `.tar`, `.tar.gz` / `.tgz`, `.tar.bz2` / `.tbz2`, `.tar.xz` / `.txz`, `.tar.zst` / `.tzst`, `.tar.lz4`

## Installation

Install [Rust](https://rustup.rs) if you don't have it, then:

```bash
cargo install --git https://github.com/bwagner/ziptree-rs
```

Or clone and build locally:

```bash
git clone https://github.com/bwagner/ziptree-rs
cd ziptree-rs
cargo install --path .
```

## Usage

```
ziptree [options] FILE
```

| Option | Description |
|--------|-------------|
| `-a`, `--all` | Show hidden files (dotfiles) |
| `-m`, `--macos` | Show `__MACOSX` metadata entries (ZIP only; `-a` not required) |
| `-s`, `--size` | Show file sizes |

## Notes

- Dotfiles are hidden by default, matching the behaviour of `tree`. Use `-a` to show them.
- `__MACOSX/` entries are ZIP-specific macOS metadata. `-m` is their sole control; `-a` is not needed alongside it.
- Works with ZIPs that omit explicit directory entries - the hierarchy is inferred from file paths.

## Comparison with `ouch list --tree`

[ouch](https://github.com/ouch-org/ouch) is a general-purpose archive tool that also supports tree output. Differences:

| | `ziptree` | `ouch list --tree` |
|---|---|---|
| Dotfiles hidden by default | yes (like `tree`) | no |
| `-a` to reveal dotfiles | yes | - |
| `-m` to reveal `__MACOSX` entries | yes | - |
| `-s` to show file sizes | yes | - |
| Dirs sorted before files | yes | no (archive order) |
| Entries sorted alphabetically | yes | no (archive order) |
| Summary line (`N directories, M files`) | yes | no |
| Formats supported | zip, tar, tar.gz, tar.bz2, tar.xz, tar.zst, tar.lz4 | zip, tar, gz, xz, bz2, zst, 7z, rar, ... |

## License

[MIT](LICENSE)
