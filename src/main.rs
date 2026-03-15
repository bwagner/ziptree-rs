use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::Path;

use bzip2::read::BzDecoder;
use clap::Parser;
use flate2::read::GzDecoder;
use xz2::read::XzDecoder;
use zip::ZipArchive;

#[derive(Parser)]
#[command(about = "Display archive contents as a tree.")]
struct Cli {
    #[arg(name = "FILE")]
    path: String,

    /// Show hidden files (dotfiles)
    #[arg(short = 'a', long = "all")]
    show_hidden: bool,

    /// Show __MACOSX metadata entries (ZIP only; -a not required)
    #[arg(short = 'm', long = "macos")]
    show_macos: bool,

    /// Show file sizes (human-readable: K, M, G, T)
    #[arg(short = 's', long = "size")]
    show_size: bool,

    /// Show file sizes in bytes (implies -s)
    #[arg(short = 'b', long = "bytes")]
    show_bytes: bool,
}

enum SizeFormat {
    None,
    Human,
    Bytes,
}

enum Format {
    Zip,
    SevenZ,
    Tar,
    TarGz,
    TarBz2,
    TarXz,
    TarZst,
    TarLz4,
}

fn detect_format(path: &str) -> Option<Format> {
    let p = path.to_lowercase();
    if p.ends_with(".zip") {
        Some(Format::Zip)
    } else if p.ends_with(".7z") {
        Some(Format::SevenZ)
    } else if p.ends_with(".tar.gz") || p.ends_with(".tgz") {
        Some(Format::TarGz)
    } else if p.ends_with(".tar.bz2") || p.ends_with(".tbz2") {
        Some(Format::TarBz2)
    } else if p.ends_with(".tar.xz") || p.ends_with(".txz") {
        Some(Format::TarXz)
    } else if p.ends_with(".tar.zst") || p.ends_with(".tzst") {
        Some(Format::TarZst)
    } else if p.ends_with(".tar.lz4") || p.ends_with(".tlz4") {
        Some(Format::TarLz4)
    } else if p.ends_with(".tar") {
        Some(Format::Tar)
    } else {
        None
    }
}

enum Node {
    Dir(HashMap<String, Node>),
    File(u64),
}

impl Node {
    fn as_dir_mut(&mut self) -> Option<&mut HashMap<String, Node>> {
        match self {
            Node::Dir(m) => Some(m),
            _ => None,
        }
    }
}

fn build_tree(names: &[(String, u64)]) -> HashMap<String, Node> {
    let mut tree: HashMap<String, Node> = HashMap::new();
    for (name, size) in names {
        let is_dir = name.ends_with('/');
        let stripped = name.trim_end_matches('/');
        let parts: Vec<&str> = stripped.split('/').filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            continue;
        }
        let mut node = &mut tree;
        for part in &parts[..parts.len() - 1] {
            let entry = node
                .entry(part.to_string())
                .or_insert_with(|| Node::Dir(HashMap::new()));
            if entry.as_dir_mut().is_none() {
                *entry = Node::Dir(HashMap::new());
            }
            node = entry.as_dir_mut().unwrap();
        }
        let last = parts[parts.len() - 1];
        if is_dir {
            node.entry(last.to_string())
                .or_insert_with(|| Node::Dir(HashMap::new()));
        } else {
            node.insert(last.to_string(), Node::File(*size));
        }
    }
    tree
}

fn human_size(n: u64) -> String {
    let units = ["B", "K", "M", "G", "T"];
    let mut val = n as f64;
    for unit in &units {
        if val < 1024.0 {
            return if *unit == "B" {
                format!("{:.0} {}", val, unit)
            } else {
                format!("{:.1} {}", val, unit)
            };
        }
        val /= 1024.0;
    }
    format!("{:.1} P", val)
}

fn bytes_size(size: u64) -> String {
    let s = size.to_string();
    let mut with_commas = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            with_commas.push(',');
        }
        with_commas.push(c);
    }
    let formatted: String = with_commas.chars().rev().collect();
    format!("[{:>12}]  ", formatted)
}

fn format_size(size: u64, fmt: &SizeFormat) -> String {
    match fmt {
        SizeFormat::Human => format!("[{:>7}]  ", human_size(size)),
        SizeFormat::Bytes => bytes_size(size),
        SizeFormat::None => String::new(),
    }
}

fn render_tree(tree: &HashMap<String, Node>, fmt: &SizeFormat, prefix: &str, out: &mut dyn Write) {
    let mut dirs: Vec<(&str, &HashMap<String, Node>)> = tree
        .iter()
        .filter_map(|(k, v)| match v {
            Node::Dir(m) => Some((k.as_str(), m)),
            _ => None,
        })
        .collect();
    dirs.sort_by_key(|(k, _)| *k);

    let mut files: Vec<(&str, u64)> = tree
        .iter()
        .filter_map(|(k, v)| match v {
            Node::File(sz) => Some((k.as_str(), *sz)),
            _ => None,
        })
        .collect();
    files.sort_by_key(|(k, _)| *k);

    let total = dirs.len() + files.len();
    let mut idx = 0;

    for (name, subtree) in &dirs {
        let is_last = idx == total - 1;
        let connector = if is_last { "└── " } else { "├── " };
        writeln!(out, "{}{}{}", prefix, connector, name).unwrap();
        let ext = if is_last { "    " } else { "│   " };
        render_tree(subtree, fmt, &format!("{}{}", prefix, ext), out);
        idx += 1;
    }

    for (name, size) in &files {
        let is_last = idx == total - 1;
        let connector = if is_last { "└── " } else { "├── " };
        writeln!(out, "{}{}{}{}", prefix, connector, format_size(*size, fmt), name).unwrap();
        idx += 1;
    }
}

fn count_tree(tree: &HashMap<String, Node>) -> (usize, usize) {
    let mut dirs = 0;
    let mut files = 0;
    for v in tree.values() {
        match v {
            Node::Dir(sub) => {
                dirs += 1;
                let (d, f) = count_tree(sub);
                dirs += d;
                files += f;
            }
            Node::File(_) => files += 1,
        }
    }
    (dirs, files)
}

fn is_hidden(name: &str, show_macos: bool) -> bool {
    if show_macos && name.starts_with("__MACOSX/") {
        return false;
    }
    name.trim_end_matches('/').split('/').any(|p| p.starts_with('.'))
}

fn read_zip_entries(path: &str) -> Result<Vec<(String, u64)>, Box<dyn std::error::Error>> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        entries.push((entry.name().to_string(), entry.size()));
    }
    Ok(entries)
}

fn read_tar_entries<R: Read>(reader: R) -> Result<Vec<(String, u64)>, Box<dyn std::error::Error>> {
    let mut archive = tar::Archive::new(reader);
    let mut entries = Vec::new();
    for entry in archive.entries()? {
        let entry = entry?;
        let raw = entry.path()?.to_string_lossy().into_owned();
        // Strip leading "./" that tar commonly adds; skip the bare "." root entry
        let path = raw.strip_prefix("./").unwrap_or(&raw).to_string();
        if path.is_empty() || path == "." {
            continue;
        }
        let is_dir = entry.header().entry_type().is_dir();
        let size = entry.header().size()?;
        if is_dir {
            let name = if path.ends_with('/') { path } else { format!("{}/", path) };
            entries.push((name, 0));
        } else {
            entries.push((path, size));
        }
    }
    Ok(entries)
}

fn read_7z_entries(path: &str) -> Result<Vec<(String, u64)>, Box<dyn std::error::Error>> {
    let reader = sevenz_rust2::SevenZReader::open(path, sevenz_rust2::Password::empty())?;
    let mut entries = Vec::new();
    for entry in &reader.archive().files {
        let name = entry.name.clone();
        if entry.is_directory {
            let name = if name.ends_with('/') { name } else { format!("{}/", name) };
            entries.push((name, 0));
        } else {
            entries.push((name, entry.size));
        }
    }
    Ok(entries)
}

fn read_entries(path: &str, format: &Format) -> Result<Vec<(String, u64)>, Box<dyn std::error::Error>> {
    let buf = || -> Result<BufReader<File>, Box<dyn std::error::Error>> {
        Ok(BufReader::new(File::open(path)?))
    };
    match format {
        Format::Zip => read_zip_entries(path),
        Format::SevenZ => read_7z_entries(path),
        Format::Tar => read_tar_entries(buf()?),
        Format::TarGz => read_tar_entries(GzDecoder::new(buf()?)),
        Format::TarBz2 => read_tar_entries(BzDecoder::new(buf()?)),
        Format::TarXz => read_tar_entries(XzDecoder::new(buf()?)),
        Format::TarZst => read_tar_entries(zstd::Decoder::new(buf()?)?),
        Format::TarLz4 => read_tar_entries(lz4_flex::frame::FrameDecoder::new(buf()?)),
    }
}

fn ziptree(
    path: &str,
    show_hidden: bool,
    show_macos: bool,
    size_format: SizeFormat,
    out: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = detect_format(path)
        .ok_or_else(|| format!("unsupported format: {}", path))?;

    let mut entries = read_entries(path, &format)?;

    if !show_macos {
        entries.retain(|(n, _)| !n.starts_with("__MACOSX/"));
    }
    if !show_hidden {
        entries.retain(|(n, _)| !is_hidden(n, show_macos));
    }

    let tree = build_tree(&entries);

    let display_name = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path);
    writeln!(out, "{}", display_name)?;
    render_tree(&tree, &size_format, "", out);

    let (dirs, files) = count_tree(&tree);
    let d = if dirs == 1 { "directory" } else { "directories" };
    let f = if files == 1 { "file" } else { "files" };
    writeln!(out, "\n{} {}, {} {}", dirs, d, files, f)?;

    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let size_format = if cli.show_bytes {
        SizeFormat::Bytes
    } else if cli.show_size {
        SizeFormat::Human
    } else {
        SizeFormat::None
    };
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if let Err(e) = ziptree(&cli.path, cli.show_hidden, cli.show_macos, size_format, &mut out) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
