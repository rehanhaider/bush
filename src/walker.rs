use crate::config::Config;
use crate::filter::Filter;
use anyhow::{bail, Result};
use ignore::WalkBuilder;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Default, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub mtime: Option<SystemTime>,
    pub mode: Option<u32>,
    pub symlink_target: Option<PathBuf>,
}

pub fn walk(root: &Path, config: &Config, filter: &Filter) -> Result<Vec<Entry>> {
    if !root.exists() {
        bail!("{}: no such file or directory", root.display());
    }

    let mut builder = WalkBuilder::new(root);
    builder
        .standard_filters(false)
        .hidden(!config.include_hidden)
        .follow_links(config.follow_symlinks);

    if let Some(d) = config.max_depth {
        builder.max_depth(Some(d));
    }

    if config.use_ignore {
        builder.filter_entry(|entry| entry.depth() == 0 || entry.file_name() != OsStr::new(".git"));
        for name in &config.ignore_files {
            builder.add_custom_ignore_filename(name);
        }
    }

    let mut entries = Vec::new();
    for result in builder.build() {
        let entry = match result {
            Ok(e) => e,
            Err(err) => {
                eprintln!("bush: {err}");
                continue;
            }
        };
        if entry.depth() == 0 {
            continue;
        }
        let file_type = entry.file_type();
        let is_dir = file_type.map(|t| t.is_dir()).unwrap_or(false);
        let is_symlink = file_type.map(|t| t.is_symlink()).unwrap_or(false);
        if config.directories_only && !is_dir {
            continue;
        }
        let path = entry.into_path();
        let rel = path.strip_prefix(root).unwrap_or(&path);
        if !filter.keep(rel, is_dir) {
            continue;
        }
        let metadata = std::fs::symlink_metadata(&path).ok();
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let mtime = metadata.as_ref().and_then(|m| m.modified().ok());
        let mode = mode_from_metadata(metadata.as_ref());
        let symlink_target = if is_symlink {
            std::fs::read_link(&path).ok()
        } else {
            None
        };
        entries.push(Entry {
            path,
            is_dir,
            is_symlink,
            size,
            mtime,
            mode,
            symlink_target,
        });
    }
    Ok(entries)
}

#[cfg(unix)]
fn mode_from_metadata(metadata: Option<&std::fs::Metadata>) -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    metadata.map(|m| m.mode())
}

#[cfg(not(unix))]
fn mode_from_metadata(_metadata: Option<&std::fs::Metadata>) -> Option<u32> {
    None
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn names(entries: &[Entry], root: &Path) -> Vec<String> {
        let mut out: Vec<String> = entries
            .iter()
            .map(|e| {
                let rel = e.path.strip_prefix(root).unwrap_or(&e.path);
                rel.components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn walks_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn walks_single_file() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("foo.txt"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert_eq!(n, vec!["foo.txt".to_string()]);
        assert!(!entries[0].is_dir);
    }

    #[test]
    fn walks_nested_dirs() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
        std::fs::write(tmp.path().join("a/b/c/deep.txt"), "x").unwrap();
        std::fs::write(tmp.path().join("top.txt"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"a".to_string()));
        assert!(n.contains(&"a/b".to_string()));
        assert!(n.contains(&"a/b/c".to_string()));
        assert!(n.contains(&"a/b/c/deep.txt".to_string()));
        assert!(n.contains(&"top.txt".to_string()));
    }

    #[test]
    fn dir_entry_marked_is_dir() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("d")).unwrap();
        std::fs::write(tmp.path().join("d/f.txt"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let d = entries.iter().find(|e| e.path.ends_with("d")).unwrap();
        let f = entries.iter().find(|e| e.path.ends_with("f.txt")).unwrap();
        assert!(d.is_dir);
        assert!(!f.is_dir);
    }

    #[test]
    fn respects_gitignore() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "ignored.txt\n").unwrap();
        std::fs::write(tmp.path().join("keep.txt"), "x").unwrap();
        std::fs::write(tmp.path().join("ignored.txt"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"keep.txt".to_string()));
        assert!(!n.contains(&"ignored.txt".to_string()));
        assert!(
            n.contains(&".gitignore".to_string()),
            "dotfiles are shown by default when not ignored"
        );
    }

    #[test]
    fn respects_multiple_ignore_files() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "git-bad\n").unwrap();
        std::fs::write(tmp.path().join(".dockerignore"), "docker-bad\n").unwrap();
        std::fs::write(tmp.path().join(".npmignore"), "npm-bad\n").unwrap();
        std::fs::write(tmp.path().join("ok.txt"), "x").unwrap();
        std::fs::write(tmp.path().join("git-bad"), "x").unwrap();
        std::fs::write(tmp.path().join("docker-bad"), "x").unwrap();
        std::fs::write(tmp.path().join("npm-bad"), "x").unwrap();
        let mut cfg = Config::default();
        cfg.ignore_files = vec![
            ".gitignore".into(),
            ".dockerignore".into(),
            ".npmignore".into(),
        ];
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"ok.txt".to_string()));
        assert!(!n.contains(&"git-bad".to_string()));
        assert!(!n.contains(&"docker-bad".to_string()));
        assert!(!n.contains(&"npm-bad".to_string()));
    }

    #[test]
    fn use_ignore_false_disables_processing() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "secret\n").unwrap();
        std::fs::write(tmp.path().join("secret"), "x").unwrap();
        let mut cfg = Config::default();
        cfg.use_ignore = false;
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"secret".to_string()));
    }

    #[test]
    fn empty_ignore_files_list_disables_filtering() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "secret\n").unwrap();
        std::fs::write(tmp.path().join("secret"), "x").unwrap();
        let mut cfg = Config::default();
        cfg.ignore_files = vec![];
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"secret".to_string()));
    }

    #[test]
    fn include_hidden_false_hides_dotfiles() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".hidden"), "x").unwrap();
        std::fs::write(tmp.path().join("visible"), "x").unwrap();
        let mut cfg = Config::default();
        cfg.include_hidden = false;
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"visible".to_string()));
        assert!(!n.contains(&".hidden".to_string()));
    }

    #[test]
    fn include_hidden_true_shows_dotfiles() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".hidden"), "x").unwrap();
        std::fs::write(tmp.path().join("visible"), "x").unwrap();
        let mut cfg = Config::default();
        cfg.include_hidden = true;
        cfg.use_ignore = false;
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"visible".to_string()));
        assert!(n.contains(&".hidden".to_string()));
    }

    #[test]
    fn default_skips_git_dir_but_shows_other_dotfiles() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        std::fs::write(tmp.path().join(".git/config"), "x").unwrap();
        std::fs::write(tmp.path().join(".npmrc"), "x").unwrap();
        std::fs::write(tmp.path().join("visible"), "x").unwrap();

        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&".npmrc".to_string()));
        assert!(n.contains(&"visible".to_string()));
        assert!(!n.contains(&".git".to_string()));
        assert!(!n.contains(&".git/config".to_string()));
    }

    #[test]
    fn use_ignore_false_disables_builtin_git_dir_skip() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        std::fs::write(tmp.path().join(".git/config"), "x").unwrap();
        let mut cfg = Config::default();
        cfg.use_ignore = false;

        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&".git".to_string()));
        assert!(n.contains(&".git/config".to_string()));
    }

    #[test]
    fn max_depth_limits_traversal() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
        std::fs::write(tmp.path().join("top.txt"), "x").unwrap();
        std::fs::write(tmp.path().join("a/mid.txt"), "x").unwrap();
        std::fs::write(tmp.path().join("a/b/c/deep.txt"), "x").unwrap();
        let mut cfg = Config::default();
        cfg.max_depth = Some(1);
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"top.txt".to_string()));
        assert!(n.contains(&"a".to_string()));
        assert!(!n.iter().any(|s| s.contains("mid.txt")));
        assert!(!n.iter().any(|s| s.contains("deep.txt")));
    }

    #[test]
    fn max_depth_zero_returns_nothing() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("top.txt"), "x").unwrap();
        let mut cfg = Config::default();
        cfg.max_depth = Some(0);
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        assert!(
            entries.is_empty(),
            "max_depth=0 should yield only the root, which is skipped"
        );
    }

    #[test]
    fn directories_only_filters_files() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("dir1")).unwrap();
        std::fs::create_dir_all(tmp.path().join("dir2")).unwrap();
        std::fs::write(tmp.path().join("file.txt"), "x").unwrap();
        std::fs::write(tmp.path().join("dir1/nested.txt"), "x").unwrap();
        let mut cfg = Config::default();
        cfg.directories_only = true;
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        assert!(entries.iter().all(|e| e.is_dir));
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"dir1".to_string()));
        assert!(n.contains(&"dir2".to_string()));
        assert!(!n.contains(&"file.txt".to_string()));
        assert!(!n.contains(&"dir1/nested.txt".to_string()));
    }

    #[test]
    fn nonexistent_root_errors_clearly() {
        let cfg = Config::default();
        let err = walk(
            Path::new("/_bush_test_does_not_exist_xyz_42"),
            &cfg,
            &Filter::default(),
        )
        .unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("no such file"), "got: {msg}");
    }

    #[test]
    fn negation_in_gitignore_works() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "*.log\n!keep.log\n").unwrap();
        std::fs::write(tmp.path().join("noisy.log"), "x").unwrap();
        std::fs::write(tmp.path().join("keep.log"), "x").unwrap();
        std::fs::write(tmp.path().join("other.txt"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"keep.log".to_string()));
        assert!(n.contains(&"other.txt".to_string()));
        assert!(!n.contains(&"noisy.log".to_string()));
    }

    #[test]
    fn nested_gitignore_applies_to_its_directory() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("sub")).unwrap();
        std::fs::write(tmp.path().join("sub/.gitignore"), "private\n").unwrap();
        std::fs::write(tmp.path().join("sub/private"), "x").unwrap();
        std::fs::write(tmp.path().join("sub/public"), "x").unwrap();
        std::fs::write(tmp.path().join("private"), "x").unwrap(); // top-level private NOT ignored
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"sub/public".to_string()));
        assert!(!n.contains(&"sub/private".to_string()));
        assert!(
            n.contains(&"private".to_string()),
            "top-level 'private' not covered by sub/.gitignore"
        );
    }

    // ===== Filesystem edge cases =====

    #[cfg(unix)]
    #[test]
    fn symlink_to_dir_not_traversed_without_follow() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("target_dir")).unwrap();
        std::fs::write(tmp.path().join("target_dir/inside.txt"), "x").unwrap();
        std::os::unix::fs::symlink(tmp.path().join("target_dir"), tmp.path().join("link")).unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"target_dir".to_string()));
        assert!(n.contains(&"target_dir/inside.txt".to_string()));
        assert!(n.contains(&"link".to_string()));
        assert!(
            !n.contains(&"link/inside.txt".to_string()),
            "should not traverse symlinked dir without --follow-symlinks"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_to_dir_traversed_with_follow() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("target_dir")).unwrap();
        std::fs::write(tmp.path().join("target_dir/inside.txt"), "x").unwrap();
        std::os::unix::fs::symlink(tmp.path().join("target_dir"), tmp.path().join("link")).unwrap();
        let mut cfg = Config::default();
        cfg.follow_symlinks = true;
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(
            n.iter().any(|s| s == "link/inside.txt"),
            "follow_symlinks should traverse symlinked dirs"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_to_file_appears_as_entry() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("real.txt"), "x").unwrap();
        std::os::unix::fs::symlink(tmp.path().join("real.txt"), tmp.path().join("link.txt"))
            .unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"real.txt".to_string()));
        assert!(n.contains(&"link.txt".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn broken_symlink_does_not_crash_walk() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("real.txt"), "x").unwrap();
        std::os::unix::fs::symlink(
            "/_bush_test_nonexistent_target_xyz",
            tmp.path().join("dangling"),
        )
        .unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(
            n.contains(&"real.txt".to_string()),
            "real sibling should still appear"
        );
    }

    #[cfg(unix)]
    #[test]
    fn permission_denied_dir_continues_walk() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let denied = tmp.path().join("denied");
        std::fs::create_dir(&denied).unwrap();
        std::fs::write(denied.join("hidden.txt"), "x").unwrap();
        std::fs::write(tmp.path().join("visible.txt"), "x").unwrap();

        std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o000)).unwrap();

        let perms_enforced = std::fs::read_dir(&denied).is_err();

        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();

        std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o755)).unwrap();

        let n = names(&entries, tmp.path());
        assert!(
            n.contains(&"visible.txt".to_string()),
            "walk must not abort on perm denied"
        );
        assert!(
            n.contains(&"denied".to_string()),
            "the denied dir itself should still be listed"
        );

        if perms_enforced {
            assert!(
                !n.contains(&"denied/hidden.txt".to_string()),
                "could not read into denied dir"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_filename_does_not_crash() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let tmp = TempDir::new().unwrap();
        let bad: &[u8] = &[b'p', b'r', b'e', 0xFF, 0xFE, b'.', b't', b'x', b't'];
        let bad_name = OsStr::from_bytes(bad);
        std::fs::write(tmp.path().join(bad_name), "x").unwrap();
        std::fs::write(tmp.path().join("normal.txt"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        assert_eq!(entries.len(), 2, "both files should be walked");
    }

    #[test]
    fn many_siblings_handled() {
        let tmp = TempDir::new().unwrap();
        for i in 0..150 {
            std::fs::write(tmp.path().join(format!("file_{i:04}.txt")), "x").unwrap();
        }
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        assert_eq!(entries.len(), 150);
    }

    #[test]
    fn deep_nesting_handled() {
        let tmp = TempDir::new().unwrap();
        let mut path = tmp.path().to_path_buf();
        for i in 0..20 {
            path = path.join(format!("d{i:02}"));
        }
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("leaf.txt"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.iter().any(|s| s.ends_with("/leaf.txt")));
        let leaf = n.iter().find(|s| s.ends_with("leaf.txt")).unwrap();
        assert_eq!(leaf.matches('/').count(), 20, "20 dir separators expected");
    }

    #[test]
    fn filename_with_spaces() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("file with spaces.txt"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"file with spaces.txt".to_string()));
    }

    #[test]
    fn unicode_filename_handled() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("café.txt"), "x").unwrap();
        std::fs::write(tmp.path().join("日本語.txt"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"café.txt".to_string()));
        assert!(n.contains(&"日本語.txt".to_string()));
    }

    // ===== Gitignore semantic edge cases =====

    #[test]
    fn trailing_slash_matches_dir_only_when_dir_exists() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "build/\n").unwrap();
        std::fs::create_dir(tmp.path().join("build")).unwrap();
        std::fs::write(tmp.path().join("build/x.txt"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(!n.iter().any(|s| s.starts_with("build")));
    }

    #[test]
    fn trailing_slash_does_not_match_file_of_same_name() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "build/\n").unwrap();
        std::fs::write(tmp.path().join("build"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(
            n.contains(&"build".to_string()),
            "file named 'build' must not be hidden by dir-only pattern 'build/'",
        );
    }

    #[test]
    fn leading_slash_anchors_to_ignore_file_root() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "/top_only\n").unwrap();
        std::fs::write(tmp.path().join("top_only"), "x").unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        std::fs::write(tmp.path().join("sub/top_only"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(!n.contains(&"top_only".to_string()));
        assert!(
            n.contains(&"sub/top_only".to_string()),
            "nested copy not anchored"
        );
    }

    #[test]
    fn double_star_matches_any_depth() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "**/secret\n").unwrap();
        std::fs::create_dir_all(tmp.path().join("a/b")).unwrap();
        std::fs::write(tmp.path().join("secret"), "x").unwrap();
        std::fs::write(tmp.path().join("a/secret"), "x").unwrap();
        std::fs::write(tmp.path().join("a/b/secret"), "x").unwrap();
        std::fs::write(tmp.path().join("notsecret"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(
            !n.iter().any(|s| s.split('/').next_back() == Some("secret")),
            "every 'secret' should be ignored, got: {n:?}"
        );
        assert!(n.contains(&"notsecret".to_string()));
    }

    #[test]
    fn comments_in_gitignore_are_skipped() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".gitignore"),
            "# header comment\nignored\n# trailing comment\n",
        )
        .unwrap();
        std::fs::write(tmp.path().join("ignored"), "x").unwrap();
        std::fs::write(tmp.path().join("keep"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(!n.contains(&"ignored".to_string()));
        assert!(n.contains(&"keep".to_string()));
    }

    #[test]
    fn blank_lines_in_gitignore_are_skipped() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "\n\n\nignored\n\n").unwrap();
        std::fs::write(tmp.path().join("ignored"), "x").unwrap();
        std::fs::write(tmp.path().join("keep"), "x").unwrap();
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(!n.contains(&"ignored".to_string()));
        assert!(n.contains(&"keep".to_string()));
    }

    #[test]
    fn two_ignore_files_rules_union() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "a\n").unwrap();
        std::fs::write(tmp.path().join(".dockerignore"), "b\n").unwrap();
        std::fs::write(tmp.path().join("a"), "x").unwrap();
        std::fs::write(tmp.path().join("b"), "x").unwrap();
        std::fs::write(tmp.path().join("c"), "x").unwrap();
        let mut cfg = Config::default();
        cfg.ignore_files.push(".dockerignore".into());
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(!n.contains(&"a".to_string()));
        assert!(!n.contains(&"b".to_string()));
        assert!(n.contains(&"c".to_string()));
    }

    // ===== ignore_files config edge cases =====

    #[test]
    fn empty_string_in_ignore_files_does_not_crash() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("x"), "x").unwrap();
        let mut cfg = Config::default();
        cfg.ignore_files = vec!["".into(), ".gitignore".into()];
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"x".to_string()));
    }

    #[test]
    fn duplicate_ignore_filenames_no_crash() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "skipme\n").unwrap();
        std::fs::write(tmp.path().join("skipme"), "x").unwrap();
        std::fs::write(tmp.path().join("keep"), "x").unwrap();
        let mut cfg = Config::default();
        cfg.ignore_files = vec![
            ".gitignore".into(),
            ".gitignore".into(),
            ".gitignore".into(),
        ];
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(!n.contains(&"skipme".to_string()));
        assert!(n.contains(&"keep".to_string()));
    }

    #[test]
    fn max_depth_very_large_is_a_noop_for_shallow_tree() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a"), "x").unwrap();
        let mut cfg = Config::default();
        cfg.max_depth = Some(1_000_000);
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn target_is_file_walks_only_that_file() {
        let tmp = TempDir::new().unwrap();
        let f = tmp.path().join("solo.txt");
        std::fs::write(&f, "x").unwrap();
        let entries = walk(&f, &Config::default(), &Filter::default()).unwrap();
        assert!(entries.is_empty(), "walking a file produces no children");
    }

    #[test]
    fn ignore_files_can_be_added() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".customignore"), "skipme\n").unwrap();
        std::fs::write(tmp.path().join("skipme"), "x").unwrap();
        std::fs::write(tmp.path().join("keepme"), "x").unwrap();

        // Without .customignore in the list: skipme is shown
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.contains(&"skipme".to_string()));

        // With .customignore added: skipme is hidden
        let mut cfg = Config::default();
        cfg.ignore_files.push(".customignore".into());
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(!n.contains(&"skipme".to_string()));
        assert!(n.contains(&"keepme".to_string()));
    }

    // ===== Additional edge cases =====

    #[cfg(unix)]
    #[test]
    fn target_is_symlink_to_dir() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("real")).unwrap();
        std::fs::write(tmp.path().join("real/inside.txt"), "x").unwrap();
        std::os::unix::fs::symlink(tmp.path().join("real"), tmp.path().join("link")).unwrap();
        // Walker should treat root specially — even without follow_symlinks,
        // descending INTO the root works.
        let entries = walk(
            &tmp.path().join("link"),
            &Config::default(),
            &Filter::default(),
        )
        .unwrap();
        let n = names(&entries, &tmp.path().join("link"));
        assert!(n.iter().any(|s| s.ends_with("inside.txt")), "got: {n:?}");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_self_loop_with_follow_terminates() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("dir1")).unwrap();
        std::fs::write(tmp.path().join("dir1/file.txt"), "x").unwrap();
        std::os::unix::fs::symlink(tmp.path().join("dir1"), tmp.path().join("dir1/self_loop"))
            .unwrap();
        let mut cfg = Config::default();
        cfg.follow_symlinks = true;
        cfg.max_depth = Some(8); // safety net so a regression can't hang the suite
        let entries = walk(tmp.path(), &cfg, &Filter::default()).unwrap();
        let n = names(&entries, tmp.path());
        assert!(n.iter().any(|s| s.ends_with("file.txt")));
    }

    #[test]
    fn stress_one_thousand_siblings() {
        let tmp = TempDir::new().unwrap();
        for i in 0..1000 {
            std::fs::write(tmp.path().join(format!("f{i:04}")), "x").unwrap();
        }
        let entries = walk(tmp.path(), &Config::default(), &Filter::default()).unwrap();
        assert_eq!(entries.len(), 1000);
    }
}
