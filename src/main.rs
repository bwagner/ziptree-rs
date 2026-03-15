use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;

use clap::Parser;
use zip::ZipArchive;

#[derive(Parser)]
#[command(about = "Display ZIP file contents as a tree.")]
struct Cli {
    #[arg(name = "FILE.zip")]
    zip_path: String,

    /// Show hidden files (dotfiles)
    #[arg(short = 'a', long = "all", name = "show_hidden")]
    show_hidden: bool,

    /// Show __MACOSX metadata entries (includes their ._* contents; -a not required)
    #[arg(short = 'm', long = "macos", name = "show_macos")]
    show_macos: bool,

    /// Show file sizes
    #[arg(short = 's', long = "size", name = "show_size")]
    show_size: bool,
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

fn format_size(size: u64) -> String {
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

fn render_tree(tree: &HashMap<String, Node>, show_size: bool, prefix: &str, out: &mut dyn Write) {
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
        let new_prefix = format!("{}{}", prefix, ext);
        render_tree(subtree, show_size, &new_prefix, out);
        idx += 1;
    }

    for (name, size) in &files {
        let is_last = idx == total - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let size_str = if show_size {
            format_size(*size)
        } else {
            String::new()
        };
        writeln!(out, "{}{}{}{}", prefix, connector, size_str, name).unwrap();
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
            Node::File(_) => {
                files += 1;
            }
        }
    }
    (dirs, files)
}

fn is_hidden(name: &str, show_macos: bool) -> bool {
    if show_macos && name.starts_with("__MACOSX/") {
        return false;
    }
    let stripped = name.trim_end_matches('/');
    stripped.split('/').any(|p| p.starts_with('.'))
}

fn ziptree(
    zip_path: &str,
    show_hidden: bool,
    show_macos: bool,
    show_size: bool,
    out: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut names: Vec<(String, u64)> = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        names.push((entry.name().to_string(), entry.size()));
    }

    if !show_macos {
        names.retain(|(n, _)| !n.starts_with("__MACOSX/"));
    }
    if !show_hidden {
        names.retain(|(n, _)| !is_hidden(n, show_macos));
    }

    let tree = build_tree(&names);

    let display_name = Path::new(zip_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(zip_path);
    writeln!(out, "{}", display_name)?;
    render_tree(&tree, show_size, "", out);

    let (dirs, files) = count_tree(&tree);
    let d = if dirs == 1 { "directory" } else { "directories" };
    let f = if files == 1 { "file" } else { "files" };
    writeln!(out, "\n{} {}, {} {}", dirs, d, files, f)?;

    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if let Err(e) = ziptree(
        &cli.zip_path,
        cli.show_hidden,
        cli.show_macos,
        cli.show_size,
        &mut out,
    ) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
