use crate::color::Colorizer;
use crate::config::SortKey;
use crate::format_meta;
use crate::walker::Entry;
use anyhow::Result;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub struct RenderOptions {
    pub colorizer: Colorizer,
    pub show_sizes: bool,
    pub show_mtime: bool,
    pub show_symlink_target: bool,
    pub show_permissions: bool,
    pub sort: SortKey,
    pub reverse: bool,
    pub prune_empty_dirs: bool,
    pub no_report: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            colorizer: Colorizer::new(crate::color::ColorMode::Never, false),
            show_sizes: false,
            show_mtime: false,
            show_symlink_target: false,
            show_permissions: false,
            sort: SortKey::Name,
            reverse: false,
            prune_empty_dirs: false,
            no_report: false,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct NodeMeta {
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub mtime: Option<SystemTime>,
    pub mode: Option<u32>,
    pub symlink_target: Option<PathBuf>,
}

#[derive(Debug, Default)]
pub struct Node {
    pub name: String,
    pub meta: NodeMeta,
    pub children: Vec<Node>,
}

pub fn prepare(root: &Path, entries: &[Entry], opts: &RenderOptions) -> Node {
    let mut tree = build(root, entries);
    if opts.prune_empty_dirs {
        prune_empty_dirs(&mut tree);
    }
    apply_sort(&mut tree, opts.sort, opts.reverse);
    tree
}

pub fn render<W: Write>(
    root: &Path,
    entries: &[Entry],
    opts: &RenderOptions,
    out: &mut W,
) -> Result<()> {
    let tree = prepare(root, entries, opts);
    render_prepared(root, &tree, opts, out)
}

pub fn render_prepared<W: Write>(
    root: &Path,
    tree: &Node,
    opts: &RenderOptions,
    out: &mut W,
) -> Result<()> {
    writeln!(out, "{}", root.display())?;
    let mut prefix = String::new();
    render_children(tree, &mut prefix, opts, out)?;
    if !opts.no_report {
        let (dirs, files) = counts(tree);
        writeln!(out)?;
        writeln!(
            out,
            "{} director{}, {} file{}",
            dirs,
            if dirs == 1 { "y" } else { "ies" },
            files,
            if files == 1 { "" } else { "s" },
        )?;
    }
    Ok(())
}

pub fn prune_empty_dirs(node: &mut Node) {
    for child in &mut node.children {
        prune_empty_dirs(child);
    }
    node.children
        .retain(|c| !c.meta.is_dir || !c.children.is_empty());
}

pub fn build(root: &Path, entries: &[Entry]) -> Node {
    let mut tree = Node {
        name: String::new(),
        meta: NodeMeta {
            is_dir: true,
            ..Default::default()
        },
        children: Vec::new(),
    };
    for entry in entries {
        let rel = entry.path.strip_prefix(root).unwrap_or(&entry.path);
        let parts: Vec<String> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        if parts.is_empty() {
            continue;
        }
        insert(&mut tree, &parts, entry);
    }
    tree
}

fn insert(node: &mut Node, parts: &[String], entry: &Entry) {
    let Some((first, rest)) = parts.split_first() else {
        return;
    };
    let idx = match node.children.iter().position(|c| c.name == *first) {
        Some(i) => i,
        None => {
            node.children.push(Node {
                name: first.clone(),
                meta: NodeMeta::default(),
                children: Vec::new(),
            });
            node.children.len() - 1
        }
    };
    let child = &mut node.children[idx];
    if rest.is_empty() {
        child.meta = NodeMeta {
            is_dir: entry.is_dir,
            is_symlink: entry.is_symlink,
            size: entry.size,
            mtime: entry.mtime,
            mode: entry.mode,
            symlink_target: entry.symlink_target.clone(),
        };
    } else {
        child.meta.is_dir = true;
        insert(child, rest, entry);
    }
}

pub fn apply_sort(node: &mut Node, key: SortKey, reverse: bool) {
    if !matches!(key, SortKey::None) {
        node.children.sort_by(|a, b| {
            let cmp = match key {
                SortKey::Name => a.name.cmp(&b.name),
                SortKey::Size => a
                    .meta
                    .size
                    .cmp(&b.meta.size)
                    .then_with(|| a.name.cmp(&b.name)),
                SortKey::Mtime => b
                    .meta
                    .mtime
                    .cmp(&a.meta.mtime)
                    .then_with(|| a.name.cmp(&b.name)),
                SortKey::None => std::cmp::Ordering::Equal,
            };
            if reverse {
                cmp.reverse()
            } else {
                cmp
            }
        });
    }
    for child in &mut node.children {
        apply_sort(child, key, reverse);
    }
}

fn render_children<W: Write>(
    node: &Node,
    prefix: &mut String,
    opts: &RenderOptions,
    out: &mut W,
) -> Result<()> {
    let total = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        let is_last = i + 1 == total;
        let connector = if is_last { "└── " } else { "├── " };
        let meta = meta_prefix(&child.meta, opts);
        let painted = opts.colorizer.paint(&child.name, &child.meta);
        let suffix = symlink_suffix(&child.meta, opts);
        writeln!(out, "{prefix}{connector}{meta}{painted}{suffix}")?;
        if !child.children.is_empty() {
            let added = if is_last { "    " } else { "│   " };
            let before = prefix.len();
            prefix.push_str(added);
            render_children(child, prefix, opts, out)?;
            prefix.truncate(before);
        }
    }
    Ok(())
}

fn meta_prefix(meta: &NodeMeta, opts: &RenderOptions) -> String {
    let mut s = String::new();
    if opts.show_permissions {
        s.push('[');
        match meta.mode {
            Some(m) => s.push_str(&format_meta::format_mode(m)),
            None => s.push_str("---------"),
        }
        s.push_str("] ");
    }
    if opts.show_sizes {
        s.push('[');
        s.push_str(&format_meta::format_size(meta.size));
        s.push_str("] ");
    }
    if opts.show_mtime {
        s.push('[');
        if let Some(t) = meta.mtime {
            s.push_str(&format_meta::format_mtime(t));
        } else {
            s.push_str("----- ----- -----");
        }
        s.push_str("] ");
    }
    s
}

fn symlink_suffix(meta: &NodeMeta, opts: &RenderOptions) -> String {
    if !opts.show_symlink_target {
        return String::new();
    }
    if !meta.is_symlink {
        return String::new();
    }
    match &meta.symlink_target {
        Some(target) => format!(" -> {}", target.display()),
        None => String::new(),
    }
}

pub fn counts(node: &Node) -> (usize, usize) {
    let mut dirs = 0;
    let mut files = 0;
    for child in &node.children {
        if child.meta.is_dir {
            dirs += 1;
        } else {
            files += 1;
        }
        let (d, f) = counts(child);
        dirs += d;
        files += f;
    }
    (dirs, files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_to_string(root: &Path, entries: &[Entry]) -> String {
        let mut out = Vec::new();
        let opts = RenderOptions::default();
        render(root, entries, &opts, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn entry(path: &str, is_dir: bool) -> Entry {
        Entry {
            path: PathBuf::from(path),
            is_dir,
            ..Default::default()
        }
    }

    #[test]
    fn empty_tree_shows_root_and_zero_counts() {
        let s = render_to_string(Path::new("/some/root"), &[]);
        assert!(s.starts_with("/some/root\n"), "got: {s}");
        assert!(s.contains("0 directories, 0 files"), "got: {s}");
    }

    #[test]
    fn single_file_uses_last_connector() {
        let s = render_to_string(Path::new("/root"), &[entry("/root/foo.txt", false)]);
        assert!(s.contains("└── foo.txt"), "got: {s}");
        assert!(s.contains("0 directories, 1 file"), "got: {s}");
    }

    #[test]
    fn multiple_siblings_use_correct_connectors() {
        let s = render_to_string(
            Path::new("/root"),
            &[
                entry("/root/a.txt", false),
                entry("/root/b.txt", false),
                entry("/root/c.txt", false),
            ],
        );
        assert!(s.contains("├── a.txt"), "got: {s}");
        assert!(s.contains("├── b.txt"), "got: {s}");
        assert!(s.contains("└── c.txt"), "got: {s}");
    }

    #[test]
    fn alphabetical_ordering_within_a_dir() {
        let s = render_to_string(
            Path::new("/r"),
            &[
                entry("/r/zebra", false),
                entry("/r/apple", false),
                entry("/r/mango", false),
            ],
        );
        let a = s.find("apple").unwrap();
        let m = s.find("mango").unwrap();
        let z = s.find("zebra").unwrap();
        assert!(a < m && m < z, "got: {s}");
    }

    #[test]
    fn nested_non_last_uses_pipe_continuation() {
        let s = render_to_string(
            Path::new("/root"),
            &[
                entry("/root/dir", true),
                entry("/root/dir/a.txt", false),
                entry("/root/dir/b.txt", false),
                entry("/root/last.txt", false),
            ],
        );
        assert!(s.contains("├── dir"), "got: {s}");
        assert!(s.contains("│   ├── a.txt"), "got: {s}");
        assert!(s.contains("│   └── b.txt"), "got: {s}");
        assert!(s.contains("└── last.txt"), "got: {s}");
    }

    #[test]
    fn nested_last_uses_space_continuation() {
        let s = render_to_string(
            Path::new("/root"),
            &[
                entry("/root/dir", true),
                entry("/root/dir/a.txt", false),
                entry("/root/dir/b.txt", false),
            ],
        );
        assert!(s.contains("└── dir"), "got: {s}");
        assert!(s.contains("    ├── a.txt"), "got: {s}");
        assert!(s.contains("    └── b.txt"), "got: {s}");
    }

    #[test]
    fn intermediate_path_creates_dir_node() {
        let s = render_to_string(Path::new("/root"), &[entry("/root/a/b/c/leaf.txt", false)]);
        assert!(s.contains("a"), "got: {s}");
        assert!(s.contains("b"), "got: {s}");
        assert!(s.contains("c"), "got: {s}");
        assert!(s.contains("leaf.txt"), "got: {s}");
        assert!(s.contains("3 directories, 1 file"), "got: {s}");
    }

    #[test]
    fn footer_singular_one_dir_one_file() {
        let s = render_to_string(
            Path::new("/r"),
            &[entry("/r/d", true), entry("/r/f", false)],
        );
        assert!(s.contains("1 directory, 1 file"), "got: {s}");
    }

    #[test]
    fn footer_plural_two_each() {
        let s = render_to_string(
            Path::new("/r"),
            &[
                entry("/r/d1", true),
                entry("/r/d2", true),
                entry("/r/f1", false),
                entry("/r/f2", false),
            ],
        );
        assert!(s.contains("2 directories, 2 files"), "got: {s}");
    }

    #[test]
    fn footer_plural_zero() {
        let s = render_to_string(Path::new("/r"), &[]);
        assert!(s.contains("0 directories, 0 files"), "got: {s}");
    }

    #[test]
    fn non_ascii_filename_renders_intact() {
        let s = render_to_string(
            Path::new("/r"),
            &[entry("/r/café.txt", false), entry("/r/日本語.md", false)],
        );
        assert!(s.contains("café.txt"), "got: {s}");
        assert!(s.contains("日本語.md"), "got: {s}");
    }

    #[test]
    fn filename_with_spaces_renders() {
        let s = render_to_string(Path::new("/r"), &[entry("/r/file with spaces.txt", false)]);
        assert!(s.contains("file with spaces.txt"), "got: {s}");
    }

    #[test]
    fn many_siblings_all_present_and_only_last_uses_corner() {
        let mut entries = Vec::new();
        for i in 0..20 {
            entries.push(entry(&format!("/r/f{i:02}"), false));
        }
        let s = render_to_string(Path::new("/r"), &entries);
        for i in 0..20 {
            assert!(s.contains(&format!("f{i:02}")), "missing f{i:02}");
        }
        let corners = s.matches("└── ").count();
        assert_eq!(corners, 1, "exactly one └── among 20 siblings");
        let tees = s.matches("├── ").count();
        assert_eq!(tees, 19, "exactly 19 ├── among 20 siblings");
    }

    #[test]
    fn deep_nesting_repeats_pipe_continuation() {
        let mut entries = Vec::new();
        let mut current = String::from("/r");
        for i in 0..6 {
            current.push_str(&format!("/d{i}"));
            entries.push(entry(&current, true));
        }
        entries.push(entry(&format!("{current}/leaf.txt"), false));
        let s = render_to_string(Path::new("/r"), &entries);
        assert!(
            s.contains("                        └── leaf.txt"),
            "got: {s}"
        );
    }

    #[test]
    fn mixed_dirs_and_files_interleave_alphabetically() {
        let s = render_to_string(
            Path::new("/r"),
            &[
                entry("/r/banana", false),
                entry("/r/apple_dir", true),
                entry("/r/cherry_dir", true),
            ],
        );
        let a = s.find("apple_dir").unwrap();
        let b = s.find("banana").unwrap();
        let c = s.find("cherry_dir").unwrap();
        assert!(a < b && b < c, "expected interleaved alphabetical");
    }

    #[test]
    fn footer_zero_dirs_one_file_uses_correct_forms() {
        let s = render_to_string(Path::new("/r"), &[entry("/r/f", false)]);
        assert!(s.contains("0 directories, 1 file"), "got: {s}");
    }

    #[test]
    fn footer_one_dir_zero_files() {
        let s = render_to_string(Path::new("/r"), &[entry("/r/d", true)]);
        assert!(s.contains("1 directory, 0 files"), "got: {s}");
    }

    #[test]
    fn root_path_appears_as_first_line() {
        let s = render_to_string(Path::new("/some/path"), &[]);
        let first_line = s.lines().next().unwrap();
        assert_eq!(first_line, "/some/path");
    }

    #[test]
    fn relative_root_renders_intact() {
        let s = render_to_string(Path::new("."), &[entry("./a.txt", false)]);
        assert!(s.starts_with(".\n"), "got: {s}");
        assert!(s.contains("a.txt"));
    }

    #[test]
    fn directories_count_includes_nested() {
        let s = render_to_string(
            Path::new("/r"),
            &[
                entry("/r/a", true),
                entry("/r/a/b", true),
                entry("/r/a/b/c", true),
            ],
        );
        assert!(s.contains("3 directories, 0 files"), "got: {s}");
    }

    // ===== Step 1 additions =====

    #[test]
    fn build_creates_node_with_metadata() {
        let entries = vec![Entry {
            path: PathBuf::from("/r/a.txt"),
            is_dir: false,
            size: 1234,
            mtime: Some(SystemTime::UNIX_EPOCH),
            mode: Some(0o100644),
            ..Default::default()
        }];
        let tree = build(Path::new("/r"), &entries);
        assert_eq!(tree.children.len(), 1);
        let leaf = &tree.children[0];
        assert_eq!(leaf.name, "a.txt");
        assert_eq!(leaf.meta.size, 1234);
        assert_eq!(leaf.meta.mode, Some(0o100644));
        assert!(leaf.meta.mtime.is_some());
    }

    #[test]
    fn build_creates_intermediate_dirs() {
        let entries = vec![Entry {
            path: PathBuf::from("/r/a/b/c.txt"),
            is_dir: false,
            ..Default::default()
        }];
        let tree = build(Path::new("/r"), &entries);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "a");
        assert!(tree.children[0].meta.is_dir);
        assert_eq!(tree.children[0].children[0].name, "b");
        assert!(tree.children[0].children[0].meta.is_dir);
    }

    #[test]
    fn build_preserves_entry_order() {
        let entries = vec![
            entry("/r/zeta", false),
            entry("/r/alpha", false),
            entry("/r/mu", false),
        ];
        let tree = build(Path::new("/r"), &entries);
        let names: Vec<&str> = tree.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["zeta", "alpha", "mu"],
            "build preserves walker order; sort happens at render"
        );
    }

    #[test]
    fn apply_sort_by_name_ascending() {
        let mut tree = build(
            Path::new("/r"),
            &[
                entry("/r/zeta", false),
                entry("/r/alpha", false),
                entry("/r/mu", false),
            ],
        );
        apply_sort(&mut tree, SortKey::Name, false);
        let names: Vec<&str> = tree.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mu", "zeta"]);
    }

    #[test]
    fn apply_sort_by_name_reversed() {
        let mut tree = build(
            Path::new("/r"),
            &[
                entry("/r/zeta", false),
                entry("/r/alpha", false),
                entry("/r/mu", false),
            ],
        );
        apply_sort(&mut tree, SortKey::Name, true);
        let names: Vec<&str> = tree.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["zeta", "mu", "alpha"]);
    }

    #[test]
    fn apply_sort_by_size_ascending() {
        let big = Entry {
            path: PathBuf::from("/r/big"),
            size: 9999,
            ..Default::default()
        };
        let small = Entry {
            path: PathBuf::from("/r/small"),
            size: 1,
            ..Default::default()
        };
        let mid = Entry {
            path: PathBuf::from("/r/mid"),
            size: 500,
            ..Default::default()
        };
        let mut tree = build(Path::new("/r"), &[big, mid, small]);
        apply_sort(&mut tree, SortKey::Size, false);
        let names: Vec<&str> = tree.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["small", "mid", "big"]);
    }

    #[test]
    fn apply_sort_by_mtime_newest_first() {
        let old = Entry {
            path: PathBuf::from("/r/old"),
            mtime: Some(SystemTime::UNIX_EPOCH),
            ..Default::default()
        };
        let new = Entry {
            path: PathBuf::from("/r/new"),
            mtime: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000)),
            ..Default::default()
        };
        let mid = Entry {
            path: PathBuf::from("/r/mid"),
            mtime: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(500_000)),
            ..Default::default()
        };
        let mut tree = build(Path::new("/r"), &[old, mid, new]);
        apply_sort(&mut tree, SortKey::Mtime, false);
        let names: Vec<&str> = tree.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["new", "mid", "old"], "newest first");
    }

    #[test]
    fn apply_sort_none_preserves_walker_order() {
        let mut tree = build(
            Path::new("/r"),
            &[
                entry("/r/zeta", false),
                entry("/r/alpha", false),
                entry("/r/mu", false),
            ],
        );
        apply_sort(&mut tree, SortKey::None, false);
        let names: Vec<&str> = tree.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["zeta", "alpha", "mu"]);
    }

    #[test]
    fn apply_sort_recursive_into_children() {
        let mut tree = build(
            Path::new("/r"),
            &[entry("/r/dir/zeta", false), entry("/r/dir/alpha", false)],
        );
        apply_sort(&mut tree, SortKey::Name, false);
        let inner: Vec<&str> = tree.children[0]
            .children
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(inner, vec!["alpha", "zeta"]);
    }

    #[test]
    fn symlink_suffix_rendered_when_enabled() {
        let entries = vec![Entry {
            path: PathBuf::from("/r/link"),
            is_symlink: true,
            symlink_target: Some(PathBuf::from("/real/target")),
            ..Default::default()
        }];
        let opts = RenderOptions {
            show_symlink_target: true,
            ..RenderOptions::default()
        };
        let mut out = Vec::new();
        render(Path::new("/r"), &entries, &opts, &mut out).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("link -> /real/target"), "got: {s}");
    }

    #[test]
    fn symlink_suffix_not_rendered_for_regular_file() {
        let entries = vec![Entry {
            path: PathBuf::from("/r/file"),
            ..Default::default()
        }];
        let opts = RenderOptions {
            show_symlink_target: true,
            ..RenderOptions::default()
        };
        let mut out = Vec::new();
        render(Path::new("/r"), &entries, &opts, &mut out).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(!s.contains(" -> "), "got: {s}");
    }

    #[test]
    fn permissions_rendered_when_enabled() {
        let entries = vec![Entry {
            path: PathBuf::from("/r/script"),
            mode: Some(0o755),
            ..Default::default()
        }];
        let opts = RenderOptions {
            show_permissions: true,
            ..RenderOptions::default()
        };
        let mut out = Vec::new();
        render(Path::new("/r"), &entries, &opts, &mut out).unwrap();
        let s = String::from_utf8(out).unwrap();
        #[cfg(unix)]
        assert!(s.contains("[rwxr-xr-x]"), "got: {s}");
        #[cfg(not(unix))]
        assert!(s.contains("[---------]"), "got: {s}");
    }

    #[test]
    fn permissions_missing_renders_dashes() {
        let entries = vec![Entry {
            path: PathBuf::from("/r/file"),
            mode: None,
            ..Default::default()
        }];
        let opts = RenderOptions {
            show_permissions: true,
            ..RenderOptions::default()
        };
        let mut out = Vec::new();
        render(Path::new("/r"), &entries, &opts, &mut out).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("[---------]"), "got: {s}");
    }

    #[test]
    fn noreport_suppresses_footer() {
        let entries = vec![entry("/r/x", false)];
        let opts = RenderOptions {
            no_report: true,
            ..RenderOptions::default()
        };
        let mut out = Vec::new();
        render(Path::new("/r"), &entries, &opts, &mut out).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(!s.contains("director"), "got: {s}");
        assert!(!s.contains("file"), "got: {s}");
    }

    #[test]
    fn prune_empty_dirs_removes_childless_dir() {
        let mut tree = Node {
            name: String::new(),
            meta: NodeMeta {
                is_dir: true,
                ..Default::default()
            },
            children: vec![
                Node {
                    name: "empty_dir".into(),
                    meta: NodeMeta {
                        is_dir: true,
                        ..Default::default()
                    },
                    children: vec![],
                },
                Node {
                    name: "file.txt".into(),
                    meta: NodeMeta::default(),
                    children: vec![],
                },
            ],
        };
        prune_empty_dirs(&mut tree);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "file.txt");
    }

    #[test]
    fn prune_keeps_dir_with_file_descendants() {
        let mut tree = Node {
            name: String::new(),
            meta: NodeMeta {
                is_dir: true,
                ..Default::default()
            },
            children: vec![Node {
                name: "kept".into(),
                meta: NodeMeta {
                    is_dir: true,
                    ..Default::default()
                },
                children: vec![Node {
                    name: "deep.txt".into(),
                    meta: NodeMeta::default(),
                    children: vec![],
                }],
            }],
        };
        prune_empty_dirs(&mut tree);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "kept");
    }

    #[test]
    fn prune_removes_nested_only_empty_dir() {
        let mut tree = Node {
            name: String::new(),
            meta: NodeMeta {
                is_dir: true,
                ..Default::default()
            },
            children: vec![Node {
                name: "outer".into(),
                meta: NodeMeta {
                    is_dir: true,
                    ..Default::default()
                },
                children: vec![Node {
                    name: "inner".into(),
                    meta: NodeMeta {
                        is_dir: true,
                        ..Default::default()
                    },
                    children: vec![],
                }],
            }],
        };
        prune_empty_dirs(&mut tree);
        assert!(
            tree.children.is_empty(),
            "no file descendant means outer dropped"
        );
    }

    #[test]
    fn apply_sort_size_with_name_tiebreak() {
        let a = Entry {
            path: PathBuf::from("/r/a"),
            size: 100,
            ..Default::default()
        };
        let b = Entry {
            path: PathBuf::from("/r/b"),
            size: 100,
            ..Default::default()
        };
        let c = Entry {
            path: PathBuf::from("/r/c"),
            size: 100,
            ..Default::default()
        };
        let mut tree = build(Path::new("/r"), &[c, a, b]);
        apply_sort(&mut tree, SortKey::Size, false);
        let names: Vec<&str> = tree.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "c"], "alphabetical tiebreak");
    }

    #[test]
    fn build_dedupes_repeated_paths() {
        let entries = vec![
            entry("/r/a/x.txt", false),
            entry("/r/a/y.txt", false),
            entry("/r/a", true),
        ];
        let tree = build(Path::new("/r"), &entries);
        assert_eq!(tree.children.len(), 1, "single 'a' node");
        assert_eq!(tree.children[0].children.len(), 2);
    }

    #[test]
    fn node_meta_carries_symlink_target() {
        let entries = vec![Entry {
            path: PathBuf::from("/r/link"),
            is_symlink: true,
            symlink_target: Some(PathBuf::from("/real/target")),
            ..Default::default()
        }];
        let tree = build(Path::new("/r"), &entries);
        assert_eq!(
            tree.children[0].meta.symlink_target,
            Some(PathBuf::from("/real/target"))
        );
        assert!(tree.children[0].meta.is_symlink);
    }
}
