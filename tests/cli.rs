//! End-to-end tests: run the ziptree binary on the archives in tests/fixtures
//! and compare its output with hand-written expected trees.
//! Regenerate the fixtures with tests/fixtures/make_fixtures.py.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_ziptree");
const FAILURE_EXIT_CODE: i32 = 1;

/// Every sample fixture except sample.tar, which leaves out big.bin.
const SAMPLE_DEFAULT: &str = "\
├── docs
│   ├── sub
│   │   └── medium.bin
│   ├── empty.txt
│   └── readme.txt
└── big.bin

2 directories, 4 files
";

const SAMPLE_TAR_DEFAULT: &str = "\
└── docs
    ├── sub
    │   └── medium.bin
    ├── empty.txt
    └── readme.txt

2 directories, 3 files
";

const SAMPLE_HUMAN_SIZES: &str = "\
├── docs
│   ├── sub
│   │   └── [  1.5 K]  medium.bin
│   ├── [    0 B]  empty.txt
│   └── [    1 B]  readme.txt
└── [  1.2 M]  big.bin

2 directories, 4 files
";

const SAMPLE_BYTE_SIZES: &str = "\
├── docs
│   ├── sub
│   │   └── [       1,536]  medium.bin
│   ├── [           0]  empty.txt
│   └── [           1]  readme.txt
└── [   1,234,567]  big.bin

2 directories, 4 files
";

const SAMPLE_ALL: &str = "\
├── .git
│   └── config
├── docs
│   ├── sub
│   │   └── medium.bin
│   ├── empty.txt
│   └── readme.txt
├── .hidden
└── big.bin

3 directories, 6 files
";

const ZIP_MACOS: &str = "\
├── __MACOSX
│   └── ._big.bin
├── docs
│   ├── sub
│   │   └── medium.bin
│   ├── empty.txt
│   └── readme.txt
└── big.bin

3 directories, 5 files
";

const ZIP_ALL_MACOS: &str = "\
├── .git
│   └── config
├── __MACOSX
│   └── ._big.bin
├── docs
│   ├── sub
│   │   └── medium.bin
│   ├── empty.txt
│   └── readme.txt
├── .hidden
└── big.bin

4 directories, 7 files
";

const SAMPLE_COMPRESSED: [&str; 7] = [
    "sample.zip",
    "sample.7z",
    "sample.tar.gz",
    "sample.tar.bz2",
    "sample.tar.xz",
    "sample.tar.zst",
    "sample.tar.lz4",
];

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// A fixture copied under a different file name; its temp directory is
/// removed on drop.
struct TempCopy {
    dir: PathBuf,
    path: PathBuf,
}

impl Drop for TempCopy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn copy_as(name: &str, new_name: &str) -> TempCopy {
    let dir =
        std::env::temp_dir().join(format!("ziptree-test-{}-{}", std::process::id(), new_name));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(new_name);
    fs::copy(fixture(name), &path).unwrap();
    TempCopy { dir, path }
}

fn run(path: &Path, args: &[&str]) -> Output {
    Command::new(BIN).args(args).arg(path).output().unwrap()
}

/// Run ziptree and assert success with the archive name followed by `body`.
fn assert_tree(path: &Path, args: &[&str], body: &str) {
    let out = run(path, args);
    assert!(
        out.status.success(),
        "{}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let name = path.file_name().unwrap().to_str().unwrap();
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("{}\n{}", name, body),
        "{} {:?}",
        name,
        args
    );
}

#[test]
fn default_output_for_each_format() {
    for name in SAMPLE_COMPRESSED {
        assert_tree(&fixture(name), &[], SAMPLE_DEFAULT);
    }
    assert_tree(&fixture("sample.tar"), &[], SAMPLE_TAR_DEFAULT);
}

#[test]
fn alias_extensions() {
    for (name, alias) in [
        ("sample.tar.gz", "sample.tgz"),
        ("sample.tar.bz2", "sample.tbz2"),
        ("sample.tar.xz", "sample.txz"),
        ("sample.tar.zst", "sample.tzst"),
        ("sample.tar.lz4", "sample.tlz4"),
    ] {
        assert_tree(&copy_as(name, alias).path, &[], SAMPLE_DEFAULT);
    }
}

#[test]
fn uppercase_extension() {
    assert_tree(
        &copy_as("sample.zip", "SAMPLE.ZIP").path,
        &[],
        SAMPLE_DEFAULT,
    );
}

#[test]
fn human_readable_sizes() {
    for name in SAMPLE_COMPRESSED {
        assert_tree(&fixture(name), &["-s"], SAMPLE_HUMAN_SIZES);
    }
}

#[test]
fn byte_sizes() {
    for name in SAMPLE_COMPRESSED {
        assert_tree(&fixture(name), &["-b"], SAMPLE_BYTE_SIZES);
    }
}

#[test]
fn bytes_take_precedence_over_human() {
    assert_tree(&fixture("sample.zip"), &["-s", "-b"], SAMPLE_BYTE_SIZES);
}

#[test]
fn all_shows_hidden_entries() {
    for name in SAMPLE_COMPRESSED {
        assert_tree(&fixture(name), &["-a"], SAMPLE_ALL);
    }
}

#[test]
fn macos_shows_macosx_without_all() {
    assert_tree(&fixture("sample.zip"), &["-m"], ZIP_MACOS);
}

#[test]
fn all_and_macos_together() {
    assert_tree(&fixture("sample.zip"), &["-a", "-m"], ZIP_ALL_MACOS);
}

#[test]
fn unsupported_extension_fails() {
    let out = run(Path::new("archive.rar"), &[]);
    assert_eq!(out.status.code(), Some(FAILURE_EXIT_CODE));
    assert!(out.stdout.is_empty());
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "error: unsupported format: archive.rar\n"
    );
}

const UTF8_TREE: &str = "\
└── caf\u{e9}
    └── na\u{ef}ve.txt

1 directory, 1 file
";

#[test]
fn utf8_names() {
    assert_tree(&fixture("enc-utf8.zip"), &[], UTF8_TREE);
    assert_tree(&fixture("enc-utf8.tar"), &[], UTF8_TREE);
}

#[test]
fn zip_cp437_name_without_utf8_flag() {
    assert_tree(
        &fixture("enc-cp437.zip"),
        &[],
        "└── caf\u{e9}.txt\n\n0 directories, 1 file\n",
    );
}

#[test]
fn tar_invalid_utf8_name_is_replaced() {
    assert_tree(
        &fixture("enc-latin1.tar"),
        &[],
        "└── caf\u{fffd}.txt\n\n0 directories, 1 file\n",
    );
}

#[test]
fn nfd_and_nfc_names_stay_distinct() {
    // NFD "e" + U+0301 sorts before NFC U+00E9 by bytes.
    let body = "├── cafe\u{301}.txt\n└── caf\u{e9}.txt\n\n0 directories, 2 files\n";
    assert_tree(&fixture("enc-nfd.zip"), &[], body);
    assert_tree(&fixture("enc-nfd.tar"), &[], body);
}

#[test]
fn sevenz_non_bmp_names() {
    assert_tree(
        &fixture("enc-unicode.7z"),
        &[],
        "└── \u{65e5}\u{672c}\u{8a9e}\n    └── \u{1f389}.txt\n\n1 directory, 1 file\n",
    );
}
