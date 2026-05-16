use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

#[derive(Default, Debug)]
pub struct Filter {
    include: Option<GlobSet>,
    exclude: Option<GlobSet>,
}

impl Filter {
    pub fn new(include_patterns: &[String], exclude_patterns: &[String]) -> Result<Self> {
        let include = build_set(include_patterns).context("compiling --include patterns")?;
        let exclude = build_set(exclude_patterns).context("compiling --exclude patterns")?;
        Ok(Self { include, exclude })
    }

    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.include.is_some() || self.exclude.is_some()
    }

    pub fn has_include(&self) -> bool {
        self.include.is_some()
    }

    /// Returns true if the given entry should be kept.
    /// - Excluded paths are always dropped.
    /// - If `--include` is active and entry is a file, it must match the include set.
    /// - Directories are always kept (they may be pruned later if they end up empty).
    pub fn keep(&self, rel_path: &Path, is_dir: bool) -> bool {
        if let Some(ex) = &self.exclude {
            for ancestor in rel_path.ancestors() {
                if ancestor.as_os_str().is_empty() {
                    break;
                }
                if ex.is_match(ancestor) {
                    return false;
                }
            }
        }
        if let Some(inc) = &self.include {
            if !is_dir && !inc.is_match(rel_path) {
                return false;
            }
        }
        true
    }
}

fn build_set(patterns: &[String]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        let glob = Glob::new(p).with_context(|| format!("invalid glob: {p:?}"))?;
        b.add(glob);
    }
    Ok(Some(b.build()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn empty_filter_is_inactive() {
        let f = Filter::new(&[], &[]).unwrap();
        assert!(!f.is_active());
        assert!(!f.has_include());
        assert!(f.keep(&PathBuf::from("any/thing"), false));
        assert!(f.keep(&PathBuf::from("any/dir"), true));
    }

    #[test]
    fn exclude_only() {
        let f = Filter::new(&[], &["*.log".into()]).unwrap();
        assert!(f.is_active());
        assert!(!f.keep(&PathBuf::from("noisy.log"), false));
        assert!(f.keep(&PathBuf::from("keep.txt"), false));
    }

    #[test]
    fn exclude_directory_by_name() {
        let f = Filter::new(&[], &["dist".into()]).unwrap();
        assert!(!f.keep(&PathBuf::from("dist"), true));
    }

    #[test]
    fn exclude_propagates_to_descendants() {
        let f = Filter::new(&[], &["vendor".into()]).unwrap();
        assert!(!f.keep(&PathBuf::from("vendor"), true));
        assert!(!f.keep(&PathBuf::from("vendor/lib.go"), false));
        assert!(!f.keep(&PathBuf::from("vendor/sub/deep.go"), false));
        assert!(f.keep(&PathBuf::from("main.go"), false));
    }

    #[test]
    fn include_only_files_matching() {
        let f = Filter::new(&["*.rs".into()], &[]).unwrap();
        assert!(f.has_include());
        assert!(f.keep(&PathBuf::from("main.rs"), false));
        assert!(!f.keep(&PathBuf::from("README.md"), false));
        assert!(f.keep(&PathBuf::from("src"), true), "dirs always kept");
    }

    #[test]
    fn exclude_overrides_include() {
        let f = Filter::new(&["*.rs".into()], &["target.rs".into()]).unwrap();
        assert!(f.keep(&PathBuf::from("main.rs"), false));
        assert!(!f.keep(&PathBuf::from("target.rs"), false));
    }

    #[test]
    fn invalid_glob_errors() {
        let err = Filter::new(&["[unclosed".into()], &[]).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("--include"), "got: {msg}");
    }

    #[test]
    fn multiple_includes_or() {
        let f = Filter::new(&["*.rs".into(), "*.toml".into()], &[]).unwrap();
        assert!(f.keep(&PathBuf::from("main.rs"), false));
        assert!(f.keep(&PathBuf::from("Cargo.toml"), false));
        assert!(!f.keep(&PathBuf::from("foo.md"), false));
    }

    #[test]
    fn nested_path_globs() {
        let f = Filter::new(&["**/*.test.js".into()], &[]).unwrap();
        assert!(f.keep(&PathBuf::from("a/b/c/x.test.js"), false));
        assert!(!f.keep(&PathBuf::from("a/b/regular.js"), false));
    }
}
