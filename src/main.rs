mod color;
mod config;
mod exit;
mod filter;
mod format;
mod format_meta;
mod tree;
mod walker;

use anyhow::{Context, Result};
use clap::Parser;
use color::{ColorMode, Colorizer};
use config::{OutputFormat, SortKey};
use filter::Filter;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use tree::RenderOptions;

#[derive(Parser, Debug)]
#[command(
    name = "bush",
    version,
    about = "A tree command substitute that respects repo ignore files"
)]
struct Cli {
    /// Path to display
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Name of an ignore file to honor (e.g. .gitignore). Repeatable; added to config list.
    #[arg(short = 'I', long = "ignore-file", value_name = "NAME")]
    ignore_file: Vec<String>,

    /// Disable all ignore processing
    #[arg(long)]
    no_ignore: bool,

    /// Force hidden (dot) files and directories to be included
    #[arg(short = 'H', long = "hidden", visible_alias = "include-hidden")]
    hidden: bool,

    /// Maximum depth to descend
    #[arg(short = 'L', long = "max-depth", value_name = "N")]
    max_depth: Option<usize>,

    /// Write output to a file instead of stdout
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Force output to stdout (overrides any output path from config)
    #[arg(long, conflicts_with = "output")]
    stdout: bool,

    /// Show directories only
    #[arg(short = 'd', long = "dirs-only")]
    dirs_only: bool,

    /// Follow symbolic links
    #[arg(long)]
    follow_symlinks: bool,

    /// Use a specific config file (skips ./.bush and ~/.bush discovery)
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Skip loading any .bush config files; use built-in defaults plus CLI flags
    #[arg(long)]
    no_config: bool,

    /// Color output (auto follows TTY)
    #[arg(long, value_enum)]
    color: Option<ColorMode>,

    /// Show file sizes
    #[arg(short = 's', long = "show-sizes")]
    show_sizes: bool,

    /// Show modification time (YYYY-MM-DD HH:MM, UTC)
    #[arg(short = 'D', long = "show-mtime")]
    show_mtime: bool,

    /// Sort key (mtime defaults to newest-first)
    #[arg(long, value_enum)]
    sort: Option<SortKey>,

    /// Reverse the sort order
    #[arg(short = 'r', long)]
    reverse: bool,

    /// Only show files matching this glob (repeatable; dirs are kept if they contain a match)
    #[arg(long = "include", value_name = "GLOB")]
    include: Vec<String>,

    /// Exclude paths matching this glob (repeatable; applies to both files and dirs)
    #[arg(long = "exclude", value_name = "GLOB")]
    exclude: Vec<String>,

    /// Suppress the "N directories, M files" footer
    #[arg(long = "noreport")]
    no_report: bool,

    /// Show symlink targets as `name -> target`
    #[arg(short = 'l', long = "show-symlink-target")]
    show_symlink_target: bool,

    /// Show file permissions (unix only)
    #[arg(short = 'p', long = "show-permissions")]
    show_permissions: bool,

    /// Output format (default: tree)
    #[arg(long, value_enum)]
    format: Option<OutputFormat>,
}

fn main() {
    if let Err(err) = run() {
        // A closed pipe (e.g. `bush | head`) is not an error worth reporting.
        if is_broken_pipe(&err) {
            std::process::exit(exit::SUCCESS);
        }
        eprintln!("Error: {err:#}");
        std::process::exit(exit::ERROR);
    }
    std::process::exit(exit::SUCCESS);
}

fn is_broken_pipe(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::BrokenPipe)
    })
}

fn run() -> Result<()> {
    let _ = ctrlc::set_handler(|| std::process::exit(exit::INTERRUPTED));

    let cli = Cli::parse();

    let search_dir = if cli.path.is_file() {
        cli.path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| cli.path.clone())
    } else {
        cli.path.clone()
    };

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|p| p.canonicalize().unwrap_or(p));
    let xdg_config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty());
    let bush_config_env = std::env::var_os("BUSH_CONFIG").map(PathBuf::from);
    let explicit_config = cli.config.clone().or(bush_config_env);
    let mut config = config::load_layered(
        explicit_config.as_deref(),
        cli.no_config,
        &search_dir,
        home.as_deref(),
        xdg_config.as_deref(),
    )?;

    if cli.no_ignore {
        config.use_ignore = false;
    }
    for name in &cli.ignore_file {
        if !config.ignore_files.contains(name) {
            config.ignore_files.push(name.clone());
        }
    }
    if cli.hidden {
        config.include_hidden = true;
    }
    if let Some(d) = cli.max_depth {
        config.max_depth = Some(d);
    }
    if let Some(o) = cli.output {
        config.output = Some(o);
    }
    if cli.stdout {
        config.output = None;
    }
    if cli.dirs_only {
        config.directories_only = true;
    }
    if cli.follow_symlinks {
        config.follow_symlinks = true;
    }
    if let Some(c) = cli.color {
        config.color = c;
    }
    if cli.show_sizes {
        config.show_sizes = true;
    }
    if cli.show_mtime {
        config.show_mtime = true;
    }
    if let Some(s) = cli.sort {
        config.sort = s;
    }
    if cli.reverse {
        config.reverse = true;
    }
    if !cli.include.is_empty() {
        config.include = cli.include;
    }
    if !cli.exclude.is_empty() {
        config.exclude = cli.exclude;
    }
    if cli.no_report {
        config.no_report = true;
    }
    if cli.show_symlink_target {
        config.show_symlink_target = true;
    }
    if cli.show_permissions {
        config.show_permissions = true;
    }
    if let Some(f) = cli.format {
        config.format = f;
    }

    let filter = Filter::new(&config.include, &config.exclude)?;
    let prune_empty_dirs = filter.has_include();
    let entries = walker::walk(&cli.path, &config, &filter)?;
    let writing_to_file = config.output.is_some();
    let is_tty =
        !writing_to_file && matches!(config.format, OutputFormat::Tree) && color::stdout_is_tty();
    let opts = RenderOptions {
        colorizer: Colorizer::new(config.color, is_tty),
        show_sizes: config.show_sizes,
        show_mtime: config.show_mtime,
        show_symlink_target: config.show_symlink_target,
        show_permissions: config.show_permissions,
        sort: config.sort,
        reverse: config.reverse,
        prune_empty_dirs,
        no_report: config.no_report,
    };

    match config.output.as_ref() {
        Some(p) => {
            let file = std::fs::File::create(p)
                .with_context(|| format!("creating output file {}", p.display()))?;
            let mut out = BufWriter::new(file);
            dispatch(&cli.path, &entries, &opts, config.format, &mut out)?;
            out.flush()?;
        }
        None => {
            let stdout = std::io::stdout();
            let mut out = BufWriter::new(stdout.lock());
            dispatch(&cli.path, &entries, &opts, config.format, &mut out)?;
            out.flush()?;
        }
    }
    Ok(())
}

fn dispatch<W: Write>(
    root: &std::path::Path,
    entries: &[walker::Entry],
    opts: &RenderOptions,
    fmt: OutputFormat,
    out: &mut W,
) -> Result<()> {
    match fmt {
        OutputFormat::Tree => tree::render(root, entries, opts, out)?,
        OutputFormat::Json => {
            let prepared = tree::prepare(root, entries, opts);
            format::render_json(root, &prepared, out)?;
        }
        OutputFormat::Html => {
            let prepared = tree::prepare(root, entries, opts);
            format::render_html(root, &prepared, out)?;
        }
        OutputFormat::Xml => {
            let prepared = tree::prepare(root, entries, opts);
            format::render_xml(root, &prepared, out)?;
        }
    }
    Ok(())
}
