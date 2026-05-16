use crate::tree::NodeMeta;
use serde::Deserialize;
use std::borrow::Cow;
use std::io::IsTerminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

pub struct Colorizer {
    enabled: bool,
}

impl Colorizer {
    pub fn new(mode: ColorMode, is_tty: bool) -> Self {
        let enabled = match mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => is_tty,
        };
        Self { enabled }
    }

    #[allow(dead_code)]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn paint<'a>(&self, name: &'a str, meta: &NodeMeta) -> Cow<'a, str> {
        if !self.enabled {
            return Cow::Borrowed(name);
        }
        match pick_style(name, meta) {
            Some(code) => Cow::Owned(format!("\x1b[{code}m{name}\x1b[0m")),
            None => Cow::Borrowed(name),
        }
    }
}

pub fn stdout_is_tty() -> bool {
    std::io::stdout().is_terminal()
}

fn pick_style(name: &str, meta: &NodeMeta) -> Option<&'static str> {
    if meta.is_symlink {
        return Some("36"); // cyan
    }
    if meta.is_dir {
        return Some("1;34"); // bold blue
    }

    #[cfg(unix)]
    if let Some(mode) = meta.mode {
        if mode & 0o111 != 0 {
            return Some("1;32"); // bold green for executable
        }
    }

    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zst" => Some("31"), // red
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "ico" | "bmp" | "tiff" | "tif" => {
            Some("35") // magenta
        }
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "opus" => Some("36"), // cyan
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "wmv" | "flv" => Some("35"), // magenta
        "rs" | "go" | "py" | "js" | "ts" | "tsx" | "jsx" | "c" | "cpp" | "cc" | "h" | "hpp"
        | "java" | "rb" | "swift" | "kt" | "scala" | "sh" | "bash" | "zsh" | "fish" => {
            Some("33") // yellow
        }
        "json" | "yaml" | "yml" | "toml" | "ini" | "conf" | "config" | "env" => Some("1;33"), // bold yellow
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn meta(is_dir: bool, is_symlink: bool, mode: Option<u32>) -> NodeMeta {
        NodeMeta {
            is_dir,
            is_symlink,
            size: 0,
            mtime: None,
            mode,
            symlink_target: None,
        }
    }

    #[test]
    fn never_mode_is_disabled() {
        let c = Colorizer::new(ColorMode::Never, true);
        assert!(!c.enabled());
        let name = c.paint("foo", &meta(true, false, None));
        assert_eq!(name, "foo", "no ANSI codes when disabled");
    }

    #[test]
    fn always_mode_is_enabled() {
        let c = Colorizer::new(ColorMode::Always, false);
        assert!(c.enabled());
    }

    #[test]
    fn auto_follows_tty() {
        assert!(Colorizer::new(ColorMode::Auto, true).enabled());
        assert!(!Colorizer::new(ColorMode::Auto, false).enabled());
    }

    #[test]
    fn directory_painted_bold_blue() {
        let c = Colorizer::new(ColorMode::Always, true);
        let s = c.paint("src", &meta(true, false, None));
        assert!(s.contains("\x1b[1;34m"), "got: {s:?}");
        assert!(s.contains("\x1b[0m"));
    }

    #[test]
    fn symlink_painted_cyan() {
        let c = Colorizer::new(ColorMode::Always, true);
        let s = c.paint("link", &meta(false, true, None));
        assert!(s.contains("\x1b[36m"), "got: {s:?}");
    }

    #[cfg(unix)]
    #[test]
    fn executable_painted_bold_green() {
        let c = Colorizer::new(ColorMode::Always, true);
        let s = c.paint("script.sh", &meta(false, false, Some(0o755)));
        // .sh extension matches yellow before exec check? Let me trace:
        // 1) is_symlink? No
        // 2) is_dir? No
        // 3) cfg unix + mode & 0o111 != 0 → yes → bold green
        assert!(s.contains("\x1b[1;32m"), "got: {s:?}");
    }

    #[test]
    fn rust_file_painted_yellow() {
        let c = Colorizer::new(ColorMode::Always, true);
        let s = c.paint("main.rs", &meta(false, false, Some(0o644)));
        assert!(s.contains("\x1b[33m"), "got: {s:?}");
    }

    #[test]
    fn config_file_painted_bold_yellow() {
        let c = Colorizer::new(ColorMode::Always, true);
        let s = c.paint("Cargo.toml", &meta(false, false, Some(0o644)));
        assert!(s.contains("\x1b[1;33m"), "got: {s:?}");
    }

    #[test]
    fn archive_painted_red() {
        let c = Colorizer::new(ColorMode::Always, true);
        let s = c.paint("data.tar.gz", &meta(false, false, Some(0o644)));
        assert!(s.contains("\x1b[31m"), "got: {s:?}");
    }

    #[test]
    fn image_painted_magenta() {
        let c = Colorizer::new(ColorMode::Always, true);
        let s = c.paint("photo.jpg", &meta(false, false, Some(0o644)));
        assert!(s.contains("\x1b[35m"), "got: {s:?}");
    }

    #[test]
    fn unknown_extension_unpainted() {
        let c = Colorizer::new(ColorMode::Always, true);
        let s = c.paint("data.unknown_ext", &meta(false, false, Some(0o644)));
        assert!(!s.contains("\x1b["), "got: {s:?}");
    }

    #[test]
    fn paint_uses_borrowed_when_no_color() {
        let c = Colorizer::new(ColorMode::Never, true);
        let s = c.paint("foo", &NodeMeta::default());
        assert!(matches!(s, Cow::Borrowed(_)));
    }

    #[test]
    fn paint_uses_owned_when_colored() {
        let c = Colorizer::new(ColorMode::Always, true);
        let s = c.paint("dir", &meta(true, false, None));
        assert!(matches!(s, Cow::Owned(_)));
    }

    #[test]
    fn case_insensitive_extension_matching() {
        let c = Colorizer::new(ColorMode::Always, true);
        let s1 = c.paint("PHOTO.JPG", &meta(false, false, Some(0o644)));
        let s2 = c.paint("photo.jpg", &meta(false, false, Some(0o644)));
        assert!(s1.contains("\x1b[35m"));
        assert!(s2.contains("\x1b[35m"));
    }

    #[test]
    fn no_extension_no_color() {
        let c = Colorizer::new(ColorMode::Always, true);
        let s = c.paint("README", &meta(false, false, Some(0o644)));
        assert!(!s.contains("\x1b["), "got: {s:?}");
    }

    // Suppress unused-import warning — PathBuf is used by future tests as we expand
    #[allow(dead_code)]
    fn _path_marker() -> PathBuf {
        PathBuf::new()
    }
}
