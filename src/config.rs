use crate::color::ColorMode;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    #[default]
    Name,
    Size,
    Mtime,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Tree,
    Json,
    Html,
    Xml,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub ignore_files: Vec<String>,
    pub include_hidden: bool,
    pub max_depth: Option<usize>,
    pub output: Option<PathBuf>,
    pub follow_symlinks: bool,
    pub directories_only: bool,
    pub use_ignore: bool,
    pub color: ColorMode,
    pub show_sizes: bool,
    pub show_mtime: bool,
    pub sort: SortKey,
    pub reverse: bool,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub no_report: bool,
    pub show_symlink_target: bool,
    pub show_permissions: bool,
    pub format: OutputFormat,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ignore_files: default_ignore_files(),
            include_hidden: true,
            max_depth: None,
            output: None,
            follow_symlinks: false,
            directories_only: false,
            use_ignore: true,
            color: ColorMode::Auto,
            show_sizes: false,
            show_mtime: false,
            sort: SortKey::Name,
            reverse: false,
            include: Vec::new(),
            exclude: Vec::new(),
            no_report: false,
            show_symlink_target: false,
            show_permissions: false,
            format: OutputFormat::Tree,
        }
    }
}

pub fn default_ignore_files() -> Vec<String> {
    vec![".gitignore".into(), ".ignore".into()]
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartialConfig {
    pub ignore_files: Option<Vec<String>>,
    pub include_hidden: Option<bool>,
    pub max_depth: Option<usize>,
    pub output: Option<PathBuf>,
    pub follow_symlinks: Option<bool>,
    pub directories_only: Option<bool>,
    pub use_ignore: Option<bool>,
    pub color: Option<ColorMode>,
    pub show_sizes: Option<bool>,
    pub show_mtime: Option<bool>,
    pub sort: Option<SortKey>,
    pub reverse: Option<bool>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub no_report: Option<bool>,
    pub show_symlink_target: Option<bool>,
    pub show_permissions: Option<bool>,
    pub format: Option<OutputFormat>,
}

impl PartialConfig {
    fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("reading config {}", path.display()))?;
        if contents.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(&contents)
            .with_context(|| format!("parsing config {}", path.display()))
    }

    fn apply(self, cfg: &mut Config) {
        if let Some(v) = self.ignore_files {
            cfg.ignore_files = v;
        }
        if let Some(v) = self.include_hidden {
            cfg.include_hidden = v;
        }
        if let Some(v) = self.max_depth {
            cfg.max_depth = Some(v);
        }
        if let Some(v) = self.output {
            cfg.output = Some(v);
        }
        if let Some(v) = self.follow_symlinks {
            cfg.follow_symlinks = v;
        }
        if let Some(v) = self.directories_only {
            cfg.directories_only = v;
        }
        if let Some(v) = self.use_ignore {
            cfg.use_ignore = v;
        }
        if let Some(v) = self.color {
            cfg.color = v;
        }
        if let Some(v) = self.show_sizes {
            cfg.show_sizes = v;
        }
        if let Some(v) = self.show_mtime {
            cfg.show_mtime = v;
        }
        if let Some(v) = self.sort {
            cfg.sort = v;
        }
        if let Some(v) = self.reverse {
            cfg.reverse = v;
        }
        if let Some(v) = self.include {
            cfg.include = v;
        }
        if let Some(v) = self.exclude {
            cfg.exclude = v;
        }
        if let Some(v) = self.no_report {
            cfg.no_report = v;
        }
        if let Some(v) = self.show_symlink_target {
            cfg.show_symlink_target = v;
        }
        if let Some(v) = self.show_permissions {
            cfg.show_permissions = v;
        }
        if let Some(v) = self.format {
            cfg.format = v;
        }
    }
}

fn find_local_config(start: &Path, home: Option<&Path>) -> Option<PathBuf> {
    let canonical = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    let mut current: Option<&Path> = Some(&canonical);
    while let Some(dir) = current {
        if home == Some(dir) {
            break;
        }
        // Prefer .bush.json over .bush when both are present.
        let json_candidate = dir.join(".bush.json");
        if json_candidate.is_file() {
            return Some(json_candidate);
        }
        let candidate = dir.join(".bush");
        if candidate.is_file() {
            return Some(candidate);
        }
        current = dir.parent();
    }
    None
}

pub fn load_layered(
    explicit: Option<&Path>,
    skip_config_files: bool,
    target_dir: &Path,
    home: Option<&Path>,
    xdg_config: Option<&Path>,
) -> Result<Config> {
    let mut cfg = Config::default();

    if let Some(path) = explicit {
        PartialConfig::load(path)?.apply(&mut cfg);
        return Ok(cfg);
    }

    if skip_config_files {
        return Ok(cfg);
    }

    // Legacy global: ~/.bush
    if let Some(h) = home {
        let home_cfg = h.join(".bush");
        if home_cfg.is_file() {
            PartialConfig::load(&home_cfg)?.apply(&mut cfg);
        }
    }

    // XDG fallback: ~/.config/bush/config.json
    if let Some(h) = home {
        let xdg_default = h.join(".config").join("bush").join("config.json");
        if xdg_default.is_file() {
            PartialConfig::load(&xdg_default)?.apply(&mut cfg);
        }
    }

    // Explicit XDG: $XDG_CONFIG_HOME/bush/config.json (takes precedence over fallback)
    if let Some(xdg) = xdg_config {
        let xdg_cfg = xdg.join("bush").join("config.json");
        if xdg_cfg.is_file() {
            PartialConfig::load(&xdg_cfg)?.apply(&mut cfg);
        }
    }

    // Local: walked-up .bush.json or .bush
    if let Some(local) = find_local_config(target_dir, home) {
        PartialConfig::load(&local)?.apply(&mut cfg);
    }

    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn canon(p: &Path) -> PathBuf {
        p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
    }

    #[test]
    fn default_config_uses_repo_oriented_ignore_files() {
        let cfg = Config::default();
        assert_eq!(cfg.ignore_files.len(), 2);
        assert!(cfg.ignore_files.contains(&".gitignore".to_string()));
        assert!(cfg.ignore_files.contains(&".ignore".to_string()));
        assert!(cfg.include_hidden);
        assert!(cfg.use_ignore);
        assert!(cfg.max_depth.is_none());
        assert!(cfg.output.is_none());
    }

    #[test]
    fn partial_config_default_is_all_none() {
        let pc = PartialConfig::default();
        assert!(pc.ignore_files.is_none());
        assert!(pc.include_hidden.is_none());
        assert!(pc.max_depth.is_none());
        assert!(pc.output.is_none());
        assert!(pc.follow_symlinks.is_none());
        assert!(pc.directories_only.is_none());
        assert!(pc.use_ignore.is_none());
    }

    #[test]
    fn apply_replaces_only_specified_fields() {
        let mut cfg = Config::default();
        let original_files = cfg.ignore_files.clone();
        let pc = PartialConfig {
            max_depth: Some(3),
            ..Default::default()
        };
        pc.apply(&mut cfg);
        assert_eq!(cfg.max_depth, Some(3));
        assert_eq!(cfg.ignore_files, original_files);
        assert!(cfg.include_hidden);
    }

    #[test]
    fn apply_replaces_ignore_files_when_specified() {
        let mut cfg = Config::default();
        let pc = PartialConfig {
            ignore_files: Some(vec![".only".into()]),
            ..Default::default()
        };
        pc.apply(&mut cfg);
        assert_eq!(cfg.ignore_files, vec![".only".to_string()]);
    }

    #[test]
    fn apply_replaces_ignore_files_with_empty_list() {
        let mut cfg = Config::default();
        let pc = PartialConfig {
            ignore_files: Some(vec![]),
            ..Default::default()
        };
        pc.apply(&mut cfg);
        assert!(cfg.ignore_files.is_empty());
    }

    #[test]
    fn apply_replaces_all_fields_when_all_specified() {
        let mut cfg = Config::default();
        let pc = PartialConfig {
            ignore_files: Some(vec![".x".into()]),
            include_hidden: Some(true),
            max_depth: Some(7),
            output: Some(PathBuf::from("out.txt")),
            follow_symlinks: Some(true),
            directories_only: Some(true),
            use_ignore: Some(false),
            color: Some(ColorMode::Always),
            show_sizes: Some(true),
            show_mtime: Some(true),
            sort: Some(SortKey::Size),
            reverse: Some(true),
            include: Some(vec!["*.rs".into()]),
            exclude: Some(vec!["target".into()]),
            no_report: Some(true),
            show_symlink_target: Some(true),
            show_permissions: Some(true),
            format: Some(OutputFormat::Json),
        };
        pc.apply(&mut cfg);
        assert_eq!(cfg.ignore_files, vec![".x".to_string()]);
        assert!(cfg.include_hidden);
        assert_eq!(cfg.max_depth, Some(7));
        assert_eq!(cfg.output, Some(PathBuf::from("out.txt")));
        assert!(cfg.follow_symlinks);
        assert!(cfg.directories_only);
        assert!(!cfg.use_ignore);
    }

    #[test]
    fn load_valid_json() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".bush");
        std::fs::write(&path, r#"{"max_depth": 4, "include_hidden": true}"#).unwrap();
        let pc = PartialConfig::load(&path).unwrap();
        assert_eq!(pc.max_depth, Some(4));
        assert_eq!(pc.include_hidden, Some(true));
        assert!(pc.ignore_files.is_none());
    }

    #[test]
    fn load_empty_json_object() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".bush");
        std::fs::write(&path, "{}").unwrap();
        let pc = PartialConfig::load(&path).unwrap();
        assert!(pc.max_depth.is_none());
    }

    #[test]
    fn load_truly_empty_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".bush");
        std::fs::write(&path, "").unwrap();
        let pc = PartialConfig::load(&path).unwrap();
        assert!(pc.ignore_files.is_none());
        assert!(pc.include_hidden.is_none());
    }

    #[test]
    fn load_whitespace_only_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".bush");
        std::fs::write(&path, "   \n  \t\n").unwrap();
        let pc = PartialConfig::load(&path).unwrap();
        assert!(pc.ignore_files.is_none());
    }

    #[test]
    fn load_invalid_json_errors() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".bush");
        std::fs::write(&path, "{ not valid json").unwrap();
        let err = PartialConfig::load(&path).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("parsing config"), "got: {msg}");
    }

    #[test]
    fn load_unknown_field_errors() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".bush");
        std::fs::write(&path, r#"{"typo_field": 1}"#).unwrap();
        let err = PartialConfig::load(&path).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("unknown field"), "got: {msg}");
    }

    #[test]
    fn load_nonexistent_file_errors() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nope.json");
        let err = PartialConfig::load(&path).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("reading config"), "got: {msg}");
    }

    #[test]
    fn find_local_config_in_start_dir() {
        let tmp = TempDir::new().unwrap();
        let bush_path = tmp.path().join(".bush");
        std::fs::write(&bush_path, "{}").unwrap();
        let found = find_local_config(tmp.path(), None).unwrap();
        assert_eq!(canon(&found), canon(&bush_path));
    }

    #[test]
    fn find_local_config_walks_up() {
        let tmp = TempDir::new().unwrap();
        let bush_path = tmp.path().join(".bush");
        std::fs::write(&bush_path, "{}").unwrap();
        let nested = tmp.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&nested).unwrap();
        let found = find_local_config(&nested, None).unwrap();
        assert_eq!(canon(&found), canon(&bush_path));
    }

    #[test]
    fn find_local_config_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        let found = find_local_config(tmp.path(), None);
        assert!(found.is_none());
    }

    #[test]
    fn find_local_config_stops_at_home() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let project = home.join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(home.join(".bush"), "{}").unwrap();
        let found = find_local_config(&project, Some(&canon(&home)));
        assert!(
            found.is_none(),
            "should not surface ~/.bush as a local config (got {found:?})"
        );
    }

    #[test]
    fn find_local_config_prefers_closer_ancestor() {
        let tmp = TempDir::new().unwrap();
        let outer_bush = tmp.path().join(".bush");
        let inner = tmp.path().join("inner");
        let inner_bush = inner.join(".bush");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(&outer_bush, r#"{"max_depth": 1}"#).unwrap();
        std::fs::write(&inner_bush, r#"{"max_depth": 9}"#).unwrap();
        let nested = inner.join("deeper");
        std::fs::create_dir_all(&nested).unwrap();
        let found = find_local_config(&nested, None).unwrap();
        assert_eq!(canon(&found), canon(&inner_bush));
    }

    #[test]
    fn load_layered_no_configs_returns_defaults() {
        let tmp = TempDir::new().unwrap();
        let cfg = load_layered(None, false, tmp.path(), None, None).unwrap();
        let default = Config::default();
        assert_eq!(cfg.ignore_files, default.ignore_files);
        assert_eq!(cfg.include_hidden, default.include_hidden);
        assert_eq!(cfg.max_depth, default.max_depth);
        assert_eq!(cfg.use_ignore, default.use_ignore);
    }

    #[test]
    fn load_layered_skip_config_files() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".bush"), r#"{"include_hidden": false}"#).unwrap();
        let cfg = load_layered(None, true, tmp.path(), None, None).unwrap();
        assert!(cfg.include_hidden);
    }

    #[test]
    fn load_layered_explicit_bypasses_discovery() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".bush"), r#"{"max_depth": 1}"#).unwrap();
        let explicit = tmp.path().join("custom.json");
        std::fs::write(&explicit, r#"{"max_depth": 9}"#).unwrap();
        let cfg = load_layered(Some(&explicit), false, tmp.path(), None, None).unwrap();
        assert_eq!(cfg.max_depth, Some(9));
    }

    #[test]
    fn load_layered_home_only() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(home.join(".bush"), r#"{"max_depth": 7}"#).unwrap();
        let cfg = load_layered(None, false, &project, Some(&canon(&home)), None).unwrap();
        assert_eq!(cfg.max_depth, Some(7));
    }

    #[test]
    fn load_layered_local_only() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(project.join(".bush"), r#"{"max_depth": 4}"#).unwrap();
        let cfg = load_layered(None, false, &project, Some(&canon(&home)), None).unwrap();
        assert_eq!(cfg.max_depth, Some(4));
    }

    #[test]
    fn load_layered_local_overrides_home() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(
            home.join(".bush"),
            r#"{"max_depth": 5, "include_hidden": true}"#,
        )
        .unwrap();
        std::fs::write(project.join(".bush"), r#"{"max_depth": 9}"#).unwrap();
        let cfg = load_layered(None, false, &project, Some(&canon(&home)), None).unwrap();
        assert_eq!(cfg.max_depth, Some(9));
        assert!(
            cfg.include_hidden,
            "home setting must survive when local doesn't override it"
        );
    }

    // ===== Config precedence matrix gaps =====

    #[test]
    fn load_layered_explicit_wins_over_skip_config() {
        let tmp = TempDir::new().unwrap();
        let explicit = tmp.path().join("custom.json");
        std::fs::write(&explicit, r#"{"max_depth": 5}"#).unwrap();
        let cfg = load_layered(Some(&explicit), true, tmp.path(), None, None).unwrap();
        assert_eq!(cfg.max_depth, Some(5));
    }

    #[test]
    fn load_layered_explicit_bypasses_home_and_local() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(home.join(".bush"), r#"{"max_depth": 1}"#).unwrap();
        std::fs::write(project.join(".bush"), r#"{"max_depth": 2}"#).unwrap();
        let explicit = tmp.path().join("custom.json");
        std::fs::write(&explicit, r#"{"max_depth": 9}"#).unwrap();
        let cfg =
            load_layered(Some(&explicit), false, &project, Some(&canon(&home)), None).unwrap();
        assert_eq!(
            cfg.max_depth,
            Some(9),
            "explicit must bypass discovery layers"
        );
    }

    #[test]
    fn load_layered_skip_config_ignores_home() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(home.join(".bush"), r#"{"max_depth": 3}"#).unwrap();
        let cfg = load_layered(None, true, &project, Some(&canon(&home)), None).unwrap();
        assert!(
            cfg.max_depth.is_none(),
            "home should not be loaded under --no-config"
        );
    }

    #[test]
    fn load_layered_skip_config_ignores_local() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".bush"), r#"{"max_depth": 3}"#).unwrap();
        let cfg = load_layered(None, true, tmp.path(), None, None).unwrap();
        assert!(
            cfg.max_depth.is_none(),
            "local must not be loaded under --no-config"
        );
    }

    #[test]
    fn load_layered_handles_no_home_env() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".bush"), r#"{"max_depth": 4}"#).unwrap();
        let cfg = load_layered(None, false, tmp.path(), None, None).unwrap();
        assert_eq!(cfg.max_depth, Some(4));
    }

    #[test]
    fn load_layered_handles_nonexistent_home() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".bush"), r#"{"max_depth": 4}"#).unwrap();
        let bogus_home = PathBuf::from("/_bush_test_no_home_xyz_42");
        let cfg = load_layered(None, false, tmp.path(), Some(&bogus_home), None).unwrap();
        assert_eq!(cfg.max_depth, Some(4));
    }

    #[test]
    fn find_local_config_handles_nonexistent_start() {
        let bogus = PathBuf::from("/_bush_test_does_not_exist_xyz_43/deeper");
        let found = find_local_config(&bogus, None);
        assert!(found.is_none());
    }

    #[test]
    fn find_local_config_handles_filesystem_root() {
        // start at "/" — should walk up to None and return None
        let found = find_local_config(Path::new("/"), None);
        assert!(found.is_none(), "expected no .bush at /; got {found:?}");
    }

    #[test]
    fn find_local_config_with_trailing_dot_in_home() {
        // Path equality treats "/home/." and "/home" as equal (CurDir component drops),
        // so trailing-dot HOME values still work as boundary.
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let project = home.join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(home.join(".bush"), "{}").unwrap();
        let with_dot = home.join(".");
        let found = find_local_config(&project, Some(&with_dot));
        assert!(
            found.is_none(),
            "trailing-dot home should still act as boundary"
        );
    }

    #[test]
    fn find_local_config_with_trailing_slash_in_home() {
        // PathBuf with trailing slash: equality normalizes, so this works as boundary.
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let project = home.join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(home.join(".bush"), "{}").unwrap();
        let with_slash = PathBuf::from(format!("{}/", home.display()));
        let found = find_local_config(&project, Some(&with_slash));
        assert!(
            found.is_none(),
            "trailing-slash home should still act as boundary"
        );
    }

    #[test]
    fn config_deserialization_rejects_negative_max_depth() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".bush");
        std::fs::write(&path, r#"{"max_depth": -5}"#).unwrap();
        let err = PartialConfig::load(&path).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("parsing config") || msg.contains("invalid"),
            "got: {msg}"
        );
    }

    #[test]
    fn config_deserialization_rejects_wrong_type_for_bool() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".bush");
        std::fs::write(&path, r#"{"include_hidden": "yes"}"#).unwrap();
        let err = PartialConfig::load(&path).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("parsing config"), "got: {msg}");
    }

    #[test]
    fn config_deserialization_rejects_wrong_type_for_array() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".bush");
        std::fs::write(&path, r#"{"ignore_files": "not_an_array"}"#).unwrap();
        let err = PartialConfig::load(&path).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("parsing config"), "got: {msg}");
    }

    #[test]
    fn config_accepts_all_seven_fields_simultaneously() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".bush");
        std::fs::write(
            &path,
            r#"{
                "ignore_files": [".foo", ".bar"],
                "include_hidden": true,
                "max_depth": 3,
                "output": "out.txt",
                "follow_symlinks": true,
                "directories_only": true,
                "use_ignore": false
            }"#,
        )
        .unwrap();
        let pc = PartialConfig::load(&path).unwrap();
        assert_eq!(
            pc.ignore_files,
            Some(vec![".foo".to_string(), ".bar".to_string()])
        );
        assert_eq!(pc.include_hidden, Some(true));
        assert_eq!(pc.max_depth, Some(3));
        assert_eq!(pc.output, Some(PathBuf::from("out.txt")));
        assert_eq!(pc.follow_symlinks, Some(true));
        assert_eq!(pc.directories_only, Some(true));
        assert_eq!(pc.use_ignore, Some(false));
    }

    #[test]
    fn load_layered_propagates_invalid_json() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".bush"), "{ bad").unwrap();
        let err = load_layered(None, false, tmp.path(), None, None).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("parsing config"), "got: {msg}");
    }

    // ===== Additional edge cases =====

    #[cfg(unix)]
    #[test]
    fn local_bush_can_be_a_symlink() {
        let tmp = TempDir::new().unwrap();
        let real = tmp.path().join("real_config.json");
        std::fs::write(&real, r#"{"max_depth": 4}"#).unwrap();
        std::os::unix::fs::symlink(&real, tmp.path().join(".bush")).unwrap();
        let cfg = load_layered(None, false, tmp.path(), None, None).unwrap();
        assert_eq!(cfg.max_depth, Some(4), "symlinked .bush should be read");
    }

    #[test]
    fn local_bush_as_directory_is_treated_as_missing() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".bush")).unwrap();
        let cfg = load_layered(None, false, tmp.path(), None, None).unwrap();
        assert_eq!(
            cfg.max_depth, None,
            ".bush as a directory should be skipped silently"
        );
        assert_eq!(cfg.ignore_files, default_ignore_files());
    }

    // ===== Discovery extension =====

    #[test]
    fn xdg_config_loaded_when_set() {
        let tmp = TempDir::new().unwrap();
        let xdg = tmp.path().join("xdg");
        std::fs::create_dir_all(xdg.join("bush")).unwrap();
        std::fs::write(xdg.join("bush/config.json"), r#"{"max_depth": 7}"#).unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        let cfg = load_layered(None, false, &project, None, Some(&xdg)).unwrap();
        assert_eq!(cfg.max_depth, Some(7));
    }

    #[test]
    fn xdg_default_loaded_from_home_dot_config() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        std::fs::create_dir_all(home.join(".config/bush")).unwrap();
        std::fs::write(home.join(".config/bush/config.json"), r#"{"max_depth": 9}"#).unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        let cfg = load_layered(None, false, &project, Some(&canon(&home)), None).unwrap();
        assert_eq!(cfg.max_depth, Some(9));
    }

    #[test]
    fn xdg_explicit_overrides_default() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        std::fs::create_dir_all(home.join(".config/bush")).unwrap();
        std::fs::write(home.join(".config/bush/config.json"), r#"{"max_depth": 1}"#).unwrap();
        let xdg = tmp.path().join("xdg");
        std::fs::create_dir_all(xdg.join("bush")).unwrap();
        std::fs::write(xdg.join("bush/config.json"), r#"{"max_depth": 5}"#).unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        let cfg = load_layered(None, false, &project, Some(&canon(&home)), Some(&xdg)).unwrap();
        assert_eq!(cfg.max_depth, Some(5));
    }

    #[test]
    fn local_overrides_xdg() {
        let tmp = TempDir::new().unwrap();
        let xdg = tmp.path().join("xdg");
        std::fs::create_dir_all(xdg.join("bush")).unwrap();
        std::fs::write(xdg.join("bush/config.json"), r#"{"max_depth": 2}"#).unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(project.join(".bush"), r#"{"max_depth": 99}"#).unwrap();
        let cfg = load_layered(None, false, &project, None, Some(&xdg)).unwrap();
        assert_eq!(cfg.max_depth, Some(99));
    }

    #[test]
    fn dot_bush_json_filename_works() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".bush.json"), r#"{"max_depth": 4}"#).unwrap();
        let cfg = load_layered(None, false, tmp.path(), None, None).unwrap();
        assert_eq!(cfg.max_depth, Some(4));
    }

    #[test]
    fn dot_bush_json_preferred_over_dot_bush() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".bush"), r#"{"max_depth": 1}"#).unwrap();
        std::fs::write(tmp.path().join(".bush.json"), r#"{"max_depth": 9}"#).unwrap();
        let cfg = load_layered(None, false, tmp.path(), None, None).unwrap();
        assert_eq!(cfg.max_depth, Some(9));
    }

    #[test]
    fn skip_config_ignores_xdg_too() {
        let tmp = TempDir::new().unwrap();
        let xdg = tmp.path().join("xdg");
        std::fs::create_dir_all(xdg.join("bush")).unwrap();
        std::fs::write(xdg.join("bush/config.json"), r#"{"max_depth": 5}"#).unwrap();
        let cfg = load_layered(None, true, tmp.path(), None, Some(&xdg)).unwrap();
        assert!(cfg.max_depth.is_none());
    }

    #[test]
    fn config_with_utf8_bom_documents_behavior() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".bush");
        // UTF-8 BOM followed by valid JSON
        let bom_content = "\u{FEFF}{\"max_depth\": 5}";
        std::fs::write(&path, bom_content).unwrap();
        // Either parses cleanly (some serde_json builds tolerate leading BOM) or
        // errors with a clear "parsing config" message. Both are acceptable; what
        // matters is no panic and predictable behavior.
        match PartialConfig::load(&path) {
            Ok(pc) => assert_eq!(pc.max_depth, Some(5)),
            Err(e) => {
                let msg = format!("{:#}", e);
                assert!(msg.contains("parsing config"), "got: {msg}");
            }
        }
    }
}
