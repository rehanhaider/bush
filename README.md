# bush

A `tree` command substitute that respects repository-oriented ignore files by default (`.gitignore` and `.ignore`) and can opt into `.dockerignore`, `.npmignore`, `.eslintignore`, `.prettierignore`, or any other ignore-file format you wire in.

Built on the same `ignore` crate that powers `ripgrep` and `fd`, so traversal is fast and the ignore semantics are correct.

```
$ bush src
src
├── color.rs
├── config.rs
├── exit.rs
├── filter.rs
├── format.rs
├── format_meta.rs
├── main.rs
├── tree.rs
└── walker.rs

0 directories, 9 files
```

## Install

```bash
git clone <this-repo> bush && cd bush
cargo build --release
ln -s "$PWD/target/release/bush" ~/.local/bin/bush     # symlink (auto-updates on rebuild)
# or:
cp     target/release/bush       ~/.local/bin/bush     # copy   (frozen at this build)
```

Verify: `bush --version` → `bush 0.3.1`

## Usage

```
bush [OPTIONS] [PATH]

  -I, --ignore-file <NAME>     Add an ignore filename to honor (repeatable)
      --no-ignore              Disable all ignore processing
  -H, --hidden                 Force dotfiles and dot-directories to be included
  -L, --max-depth <N>          Limit traversal depth
  -o, --output <FILE>          Write to FILE instead of stdout
      --stdout                 Force stdout (overrides any output= in config)
  -d, --dirs-only              Show only directories
      --follow-symlinks        Traverse into symlinked directories
      --color <auto|always|never>
                               Color output (auto follows TTY)
  -s, --show-sizes             Show file sizes in [bracketed] form
  -D, --show-mtime             Show modification time (YYYY-MM-DD HH:MM, UTC)
  -p, --show-permissions       Show file mode (unix; e.g. [rwxr-xr-x])
  -l, --show-symlink-target    Print "name -> target" for symlinks
      --sort <name|size|mtime|none>
                               Sort key (default name; mtime is newest-first)
  -r, --reverse                Reverse the sort order
      --include <GLOB>         Only show files matching this glob (repeatable)
      --exclude <GLOB>         Exclude paths matching this glob (repeatable)
      --noreport               Suppress the "N directories, M files" footer
      --format <tree|json|html|xml>
                               Output format (default tree)
      --config <FILE>          Use a specific config file (skips discovery)
      --no-config              Skip .bush config discovery; use defaults + CLI
  -h, --help                   Print help
  -V, --version                Print version
```

### Examples

```bash
bush                          # current directory, defaults
bush src/ -L 2 -d             # 2 levels, directories only
bush --no-ignore              # show ignored paths too
bush -I .dockerignore         # also honor Docker build-context ignores
bush -s -D -p                 # size + mtime + permissions
bush --sort size -r           # largest files first
bush --include "*.rs"         # only Rust files (parent dirs auto-pruned)
bush --exclude target         # hide target/ entirely
bush --format json | jq       # structured output for scripting
bush --color=always | less -R # force color through pipes
bush -o STRUCTURE.txt         # write to file
```

## Configuration

bush reads JSON config from several locations, merging them in a defined precedence. All keys are optional; unknown keys produce a clear error so typos surface immediately.

`.bush` (or `.bush.json`) example:

```json
{
  "ignore_files": [".gitignore", ".ignore"],
  "include_hidden": true,
  "max_depth": 3,
  "output": "STRUCTURE.txt",
  "follow_symlinks": false,
  "directories_only": false,
  "use_ignore": true,
  "color": "auto",
  "show_sizes": false,
  "show_mtime": false,
  "show_permissions": false,
  "show_symlink_target": false,
  "sort": "name",
  "reverse": false,
  "include": [],
  "exclude": [],
  "no_report": false,
  "format": "tree"
}
```

### Discovery & precedence (highest wins)

```
CLI flags
  > local .bush.json or .bush       (walked up from target dir)
  > $XDG_CONFIG_HOME/bush/config.json
  > ~/.config/bush/config.json
  > ~/.bush                         (legacy global)
  > built-in defaults
```

- `--config <FILE>` bypasses every discovery layer; CLI flags still layer on top.
- `BUSH_CONFIG=<path>` env var acts like `--config` but loses to an explicit `--config` flag.
- `--no-config` skips all discovery; built-in defaults still apply.
- Local lookup walks **up** from the target directory and stops at `$HOME` so `~/.bush` is treated as global, not local.
- `.bush.json` is preferred over `.bush` when both exist in the same directory.

### Built-in defaults

If no config is found, `bush` honors:

- `.gitignore`
- `.ignore` (the ripgrep/fd convention)

It also prunes `.git/` by default so including dotfiles does not expand the repository database. Use `--no-ignore` to disable ignore processing entirely.

Ignore files in **ancestor directories** of the target are honored too, so `bush src/` applies the repository-root `.gitignore` the same way `git`, `rg`, and `fd` do.

Other ignore-file formats are opt-in:

```bash
bush -I .dockerignore
bush -I .npmignore -I .eslintignore
```

### Color palette (LS_COLORS-style)

| Type / extension                                 | Style       |
|--------------------------------------------------|-------------|
| Directories                                      | bold blue   |
| Symlinks                                         | cyan        |
| Executables (`mode & 0o111`)                     | bold green  |
| `.zip .tar .gz .bz2 .xz .7z .rar .zst`           | red         |
| `.jpg .png .gif .webp .svg .bmp .tif`            | magenta     |
| `.mp3 .wav .flac .ogg .m4a`                      | cyan        |
| `.mp4 .mkv .avi .mov .webm`                      | magenta     |
| `.rs .go .py .js .ts .c .cpp .java .rb .swift …` | yellow      |
| `.json .yaml .toml .ini .conf .env`              | bold yellow |
| Anything else                                    | default     |

## Exit codes

| Code | Meaning                                                          |
|------|------------------------------------------------------------------|
| 0    | Success (including output cut short by a closed pipe, e.g. `bush \| head`) |
| 1    | Runtime error (invalid JSON in config, nonexistent path, unknown config key, glob compile error, file write error, …) |
| 2    | CLI parse error (handled by clap)                                |
| 130  | Interrupted by SIGINT (Ctrl-C)                                   |

## Testing

```bash
cargo test               # ~275 tests across unit + integration
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```

## License

MIT — see [LICENSE](LICENSE).
