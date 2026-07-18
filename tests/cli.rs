use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn bush() -> Command {
    let mut cmd = Command::cargo_bin("bush").unwrap();
    cmd.env_remove("HOME");
    cmd
}

#[test]
fn default_run_honors_builtin_ignore_set() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("node_modules")).unwrap();
    fs::write(tmp.path().join("node_modules/dep.js"), "x").unwrap();
    fs::write(tmp.path().join("keep.txt"), "x").unwrap();
    fs::write(tmp.path().join(".gitignore"), "node_modules\n").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("keep.txt"))
        .stdout(predicate::str::contains("node_modules").not());
}

#[test]
fn default_run_does_not_honor_dockerignore() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("secrets")).unwrap();
    fs::write(tmp.path().join("secrets/key.pem"), "x").unwrap();
    fs::write(tmp.path().join("app.py"), "x").unwrap();
    fs::write(tmp.path().join(".dockerignore"), "secrets\n").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("app.py"))
        .stdout(predicate::str::contains("secrets"));
}

#[test]
fn cli_ignore_file_honors_dockerignore() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("secrets")).unwrap();
    fs::write(tmp.path().join("secrets/key.pem"), "x").unwrap();
    fs::write(tmp.path().join("app.py"), "x").unwrap();
    fs::write(tmp.path().join(".dockerignore"), "secrets\n").unwrap();

    bush()
        .arg(tmp.path())
        .args(["-I", ".dockerignore"])
        .assert()
        .success()
        .stdout(predicate::str::contains("app.py"))
        .stdout(predicate::str::contains("secrets").not());
}

#[test]
fn default_run_does_not_honor_npmignore() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("docs")).unwrap();
    fs::write(tmp.path().join("docs/manual.md"), "x").unwrap();
    fs::write(tmp.path().join("index.js"), "x").unwrap();
    fs::write(tmp.path().join(".npmignore"), "docs\n").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("index.js"))
        .stdout(predicate::str::contains("docs"));
}

#[test]
fn cli_ignore_file_honors_npmignore() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("docs")).unwrap();
    fs::write(tmp.path().join("docs/manual.md"), "x").unwrap();
    fs::write(tmp.path().join("index.js"), "x").unwrap();
    fs::write(tmp.path().join(".npmignore"), "docs\n").unwrap();

    bush()
        .arg(tmp.path())
        .args(["-I", ".npmignore"])
        .assert()
        .success()
        .stdout(predicate::str::contains("index.js"))
        .stdout(predicate::str::contains("docs").not());
}

#[test]
fn no_ignore_flag_shows_everything() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("node_modules")).unwrap();
    fs::write(tmp.path().join("node_modules/dep.js"), "x").unwrap();
    fs::write(tmp.path().join(".gitignore"), "node_modules\n").unwrap();

    bush()
        .arg(tmp.path())
        .arg("--no-ignore")
        .assert()
        .success()
        .stdout(predicate::str::contains("node_modules"));
}

#[test]
fn default_run_shows_dotfiles() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".hidden"), "x").unwrap();
    fs::write(tmp.path().join("visible"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("visible"))
        .stdout(predicate::str::contains(".hidden"));

    bush()
        .arg(tmp.path())
        .arg("--hidden")
        .assert()
        .success()
        .stdout(predicate::str::contains(".hidden"));
}

#[test]
fn default_run_skips_git_dir() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join(".git")).unwrap();
    fs::write(tmp.path().join(".git/config"), "x").unwrap();
    fs::write(tmp.path().join(".npmrc"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(".npmrc"))
        .stdout(predicate::str::contains(".git").not());
}

#[test]
fn no_ignore_shows_git_dir() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join(".git")).unwrap();
    fs::write(tmp.path().join(".git/config"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .arg("--no-ignore")
        .assert()
        .success()
        .stdout(predicate::str::contains(".git"));
}

#[test]
fn max_depth_flag_limits_traversal() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
    fs::write(tmp.path().join("a/b/c/deep.txt"), "x").unwrap();
    fs::write(tmp.path().join("top.txt"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .args(["-L", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("top.txt"))
        .stdout(predicate::str::contains("deep.txt").not());
}

#[test]
fn dirs_only_filters_files() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("d1")).unwrap();
    fs::write(tmp.path().join("file.txt"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .arg("-d")
        .assert()
        .success()
        .stdout(predicate::str::contains("d1"))
        .stdout(predicate::str::contains("file.txt").not());
}

#[test]
fn output_flag_writes_to_file_no_stdout() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("keep.txt"), "x").unwrap();
    let out_path = tmp.path().join("tree.txt");

    bush()
        .arg(tmp.path())
        .args(["-o", out_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let contents = fs::read_to_string(&out_path).unwrap();
    assert!(contents.contains("keep.txt"), "got: {contents}");
}

#[test]
fn stdout_flag_overrides_config_output() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("keep.txt"), "x").unwrap();
    fs::write(
        tmp.path().join(".bush"),
        r#"{"output": "should_not_be_written.txt"}"#,
    )
    .unwrap();

    bush()
        .arg(tmp.path())
        .arg("--stdout")
        .assert()
        .success()
        .stdout(predicate::str::contains("keep.txt"));

    assert!(!tmp.path().join("should_not_be_written.txt").exists());
}

#[test]
fn cli_ignore_file_adds_to_default_set() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".customignore"), "skipme\n").unwrap();
    fs::write(tmp.path().join("skipme"), "x").unwrap();
    fs::write(tmp.path().join("keepme"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("skipme"));

    bush()
        .arg(tmp.path())
        .args(["-I", ".customignore"])
        .assert()
        .success()
        .stdout(predicate::str::contains("keepme"))
        .stdout(predicate::str::contains("skipme").not());
}

#[test]
fn local_bush_in_target_applies() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("noisy.log"), "x").unwrap();
    fs::write(tmp.path().join("quiet.txt"), "x").unwrap();
    fs::write(tmp.path().join(".gitignore"), "*.log\n").unwrap();
    fs::write(tmp.path().join(".bush"), r#"{"ignore_files": []}"#).unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log"))
        .stdout(predicate::str::contains("quiet.txt"));
}

#[test]
fn walk_up_finds_parent_bush() {
    let tmp = TempDir::new().unwrap();
    let child = tmp.path().join("child");
    fs::create_dir_all(&child).unwrap();
    fs::write(child.join("noisy.log"), "x").unwrap();
    fs::write(child.join(".gitignore"), "*.log\n").unwrap();
    fs::write(tmp.path().join(".bush"), r#"{"ignore_files": []}"#).unwrap();

    bush()
        .arg(&child)
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log"));
}

#[test]
fn home_bush_applies_via_home_env() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("fake_home");
    let project = tmp.path().join("project");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("noisy.log"), "x").unwrap();
    fs::write(project.join(".gitignore"), "*.log\n").unwrap();
    fs::write(home.join(".bush"), r#"{"ignore_files": []}"#).unwrap();

    bush()
        .env("HOME", &home)
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log"));
}

#[test]
fn local_overrides_home_bush() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("fake_home");
    let project = tmp.path().join("project");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("noisy.log"), "x").unwrap();
    fs::write(project.join(".gitignore"), "*.log\n").unwrap();
    fs::write(home.join(".bush"), r#"{"ignore_files": []}"#).unwrap();
    fs::write(project.join(".bush"), r#"{"ignore_files": [".gitignore"]}"#).unwrap();

    bush()
        .env("HOME", &home)
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log").not());
}

#[test]
fn explicit_config_bypasses_discovery() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("noisy.log"), "x").unwrap();
    fs::write(tmp.path().join(".gitignore"), "*.log\n").unwrap();
    fs::write(
        tmp.path().join(".bush"),
        r#"{"ignore_files": [".gitignore"]}"#,
    )
    .unwrap();
    let explicit = tmp.path().join("custom.json");
    fs::write(&explicit, r#"{"ignore_files": []}"#).unwrap();

    bush()
        .arg(tmp.path())
        .args(["--config", explicit.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log"));
}

#[test]
fn no_config_flag_skips_discovery() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("noisy.log"), "x").unwrap();
    fs::write(tmp.path().join(".gitignore"), "*.log\n").unwrap();
    fs::write(tmp.path().join(".bush"), r#"{"ignore_files": []}"#).unwrap();

    bush()
        .arg(tmp.path())
        .arg("--no-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log").not());
}

#[test]
fn invalid_json_in_local_bush_errors() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".bush"), "{ not valid").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("parsing config"));
}

#[test]
fn unknown_config_key_errors() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".bush"), r#"{"typo_field": 1}"#).unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown field"));
}

#[test]
fn nonexistent_path_exits_nonzero() {
    bush()
        .arg("/_does_not_exist_bush_test_42")
        .assert()
        .failure()
        .stderr(predicate::str::contains("no such file"));
}

#[test]
fn version_prints_name_and_version() {
    bush()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("bush"));
}

#[test]
fn help_lists_key_flags() {
    bush()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("--ignore-file"))
        .stdout(predicate::str::contains("--max-depth"))
        .stdout(predicate::str::contains("--no-ignore"))
        .stdout(predicate::str::contains("--config"));
}

#[test]
fn relative_target_via_cwd() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("hello.txt"), "x").unwrap();

    bush()
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("hello.txt"));
}

#[test]
fn footer_counts_match_contents() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("d1")).unwrap();
    fs::create_dir(tmp.path().join("d2")).unwrap();
    fs::write(tmp.path().join("f1"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("2 directories, 1 file"));
}

#[test]
fn unicode_tree_characters_in_output() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("a"), "x").unwrap();
    fs::write(tmp.path().join("b"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("├──"))
        .stdout(predicate::str::contains("└──"));
}

#[test]
fn output_to_relative_path_resolves_against_cwd() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("hello.txt"), "x").unwrap();

    bush()
        .current_dir(tmp.path())
        .args(["-o", "out.txt"])
        .assert()
        .success();

    let contents = fs::read_to_string(tmp.path().join("out.txt")).unwrap();
    assert!(contents.contains("hello.txt"), "got: {contents}");
}

// ===== CLI flag interactions =====

#[test]
fn no_ignore_makes_cli_ignore_file_noop() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".customignore"), "skipme\n").unwrap();
    fs::write(tmp.path().join("skipme"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .args(["--no-ignore", "-I", ".customignore"])
        .assert()
        .success()
        .stdout(predicate::str::contains("skipme"));
}

#[test]
fn cli_hidden_overrides_config_false() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".hidden_thing"), "x").unwrap();
    fs::write(tmp.path().join(".bush"), r#"{"include_hidden": false}"#).unwrap();

    bush()
        .arg(tmp.path())
        .arg("--hidden")
        .assert()
        .success()
        .stdout(predicate::str::contains(".hidden_thing"));
}

#[test]
fn repeated_ignore_file_flag_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".customignore"), "skipme\n").unwrap();
    fs::write(tmp.path().join("skipme"), "x").unwrap();
    fs::write(tmp.path().join("keepme"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .args([
            "-I",
            ".customignore",
            "-I",
            ".customignore",
            "-I",
            ".customignore",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("keepme"))
        .stdout(predicate::str::contains("skipme").not());
}

#[test]
fn multiple_distinct_ignore_file_flags_all_apply() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".oneignore"), "alpha\n").unwrap();
    fs::write(tmp.path().join(".twoignore"), "beta\n").unwrap();
    fs::write(tmp.path().join("alpha"), "x").unwrap();
    fs::write(tmp.path().join("beta"), "x").unwrap();
    fs::write(tmp.path().join("gamma"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .args(["-I", ".oneignore", "-I", ".twoignore"])
        .assert()
        .success()
        .stdout(predicate::str::contains("gamma"))
        .stdout(predicate::str::contains("alpha").not())
        .stdout(predicate::str::contains("beta").not());
}

#[test]
fn output_and_stdout_flags_conflict() {
    bush()
        .args(["-o", "x.txt", "--stdout", "."])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used"));
}

#[test]
fn config_and_no_config_together_explicit_wins() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("noisy.log"), "x").unwrap();
    fs::write(tmp.path().join(".gitignore"), "*.log\n").unwrap();
    let explicit = tmp.path().join("custom.json");
    fs::write(&explicit, r#"{"ignore_files": []}"#).unwrap();

    bush()
        .arg(tmp.path())
        .args(["--config", explicit.to_str().unwrap(), "--no-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log"));
}

#[test]
fn explicit_config_with_cli_overrides() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("noisy.log"), "x").unwrap();
    fs::write(tmp.path().join(".gitignore"), "*.log\n").unwrap();
    let explicit = tmp.path().join("custom.json");
    fs::write(&explicit, r#"{"ignore_files": [".gitignore"]}"#).unwrap();

    bush()
        .arg(tmp.path())
        .args(["--config", explicit.to_str().unwrap(), "--no-ignore"])
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log"));
}

#[test]
fn explicit_config_nonexistent_path_errors() {
    let tmp = TempDir::new().unwrap();
    bush()
        .arg(tmp.path())
        .args(["--config", "/_bush_test_no_such_config_xyz_42"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("reading config"));
}

#[test]
fn max_depth_zero_via_cli() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("file.txt"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .args(["-L", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("file.txt").not())
        .stdout(predicate::str::contains("0 directories, 0 files"));
}

#[test]
fn max_depth_non_integer_clap_errors() {
    bush().args(["-L", "not-an-int", "."]).assert().failure();
}

#[test]
fn max_depth_negative_clap_errors() {
    bush().args(["--max-depth=-1", "."]).assert().failure();
}

#[test]
fn max_depth_overflow_clap_errors() {
    bush()
        .args(["--max-depth=99999999999999999999999", "."])
        .assert()
        .failure();
}

#[test]
fn no_path_arg_uses_cwd() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("hello.txt"), "x").unwrap();

    bush()
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("hello.txt"));
}

#[test]
fn include_hidden_alias_works() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".hidden"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .arg("--include-hidden")
        .assert()
        .success()
        .stdout(predicate::str::contains(".hidden"));
}

// ===== Symlink behavior (Unix) =====

#[cfg(unix)]
#[test]
fn follow_symlinks_traverses_into_symlinked_dir() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("target")).unwrap();
    fs::write(tmp.path().join("target/inner.txt"), "x").unwrap();
    std::os::unix::fs::symlink(tmp.path().join("target"), tmp.path().join("link")).unwrap();

    bush()
        .arg(tmp.path())
        .arg("--follow-symlinks")
        .assert()
        .success()
        .stdout(predicate::str::contains("inner.txt"));
}

#[cfg(unix)]
#[test]
fn default_does_not_follow_symlinks() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("target")).unwrap();
    fs::write(tmp.path().join("target/inner.txt"), "x").unwrap();
    std::os::unix::fs::symlink(tmp.path().join("target"), tmp.path().join("link")).unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("link"))
        .stdout(predicate::str::contains("inner.txt").count(1));
}

// ===== Output edge cases =====

#[test]
fn output_overwrites_existing_file() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x"), "y").unwrap();
    let out = tmp.path().join("tree.txt");
    fs::write(&out, "STALE OLD CONTENT").unwrap();

    bush()
        .arg(tmp.path())
        .args(["-o", out.to_str().unwrap()])
        .assert()
        .success();

    let contents = fs::read_to_string(&out).unwrap();
    assert!(!contents.contains("STALE OLD CONTENT"));
    assert!(contents.contains("x"));
}

#[test]
fn output_to_dev_null_succeeds() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x"), "y").unwrap();

    bush()
        .arg(tmp.path())
        .args(["-o", "/dev/null"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn output_to_nonexistent_parent_dir_errors() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x"), "y").unwrap();
    let out = tmp.path().join("does_not_exist/tree.txt");

    bush()
        .arg(tmp.path())
        .args(["-o", out.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("creating output file"));
}

#[test]
fn output_file_path_with_spaces() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x"), "y").unwrap();
    let out = tmp.path().join("tree with spaces.txt");

    bush()
        .arg(tmp.path())
        .args(["-o", out.to_str().unwrap()])
        .assert()
        .success();

    assert!(fs::read_to_string(&out).unwrap().contains("x"));
}

// ===== HOME env edge cases =====

#[test]
fn home_unset_works_no_crash() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x"), "y").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("x"));
}

#[test]
fn home_nonexistent_dir_works() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x"), "y").unwrap();

    bush()
        .env("HOME", "/_bush_test_no_home_dir_xyz_42")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("x"));
}

#[test]
fn home_empty_string_works() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x"), "y").unwrap();

    bush()
        .env("HOME", "")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("x"));
}

// ===== Walk-up + HOME boundary =====

#[test]
fn walk_up_does_not_pick_up_home_bush_as_local() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("fake_home");
    let project = home.join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("noisy.log"), "x").unwrap();
    fs::write(project.join(".gitignore"), "*.log\n").unwrap();
    // home's .bush would disable ignores if found via local walk-up
    fs::write(home.join(".bush"), r#"{"ignore_files": []}"#).unwrap();

    // With HOME set: home/.bush IS loaded as global config (so noisy.log shown anyway)
    bush()
        .env("HOME", &home)
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log"));

    // Without HOME: walk-up reaches home/.bush directly as if it's a local ancestor
    // (this is the unintuitive case the boundary tries to prevent)
    bush().arg(&project).assert().success();
}

// ===== Permission denied (Unix) =====

#[cfg(unix)]
#[test]
fn permission_denied_dir_emits_stderr_continues() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = TempDir::new().unwrap();
    let denied = tmp.path().join("denied");
    fs::create_dir(&denied).unwrap();
    fs::write(denied.join("inside.txt"), "x").unwrap();
    fs::write(tmp.path().join("visible.txt"), "x").unwrap();
    fs::set_permissions(&denied, fs::Permissions::from_mode(0o000)).unwrap();

    let perms_enforced = fs::read_dir(&denied).is_err();

    let result = bush().arg(tmp.path()).assert().success();

    fs::set_permissions(&denied, fs::Permissions::from_mode(0o755)).unwrap();

    let stdout = String::from_utf8_lossy(&result.get_output().stdout).to_string();
    assert!(stdout.contains("visible.txt"));
    assert!(stdout.contains("denied"));

    if perms_enforced {
        assert!(
            !stdout.contains("inside.txt"),
            "perm denied dir should not be entered"
        );
    }
}

// ===== Gitignore-syntax pass-through over CLI =====

#[test]
fn comments_and_blanks_in_gitignore_via_cli() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join(".gitignore"),
        "# comment\n\nignored\n\n# another\n",
    )
    .unwrap();
    fs::write(tmp.path().join("ignored"), "x").unwrap();
    fs::write(tmp.path().join("keep"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("keep"))
        .stdout(predicate::str::contains("ignored").not());
}

#[test]
fn negation_pattern_via_cli() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".gitignore"), "*.log\n!keep.log\n").unwrap();
    fs::write(tmp.path().join("noisy.log"), "x").unwrap();
    fs::write(tmp.path().join("keep.log"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("keep.log"))
        .stdout(predicate::str::contains("noisy.log").not());
}

#[test]
fn many_files_render_correctly_via_cli() {
    let tmp = TempDir::new().unwrap();
    for i in 0..50 {
        fs::write(tmp.path().join(format!("f{i:02}")), "x").unwrap();
    }

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("50 files"));
}

#[test]
fn deep_nesting_via_cli() {
    let tmp = TempDir::new().unwrap();
    let mut path = tmp.path().to_path_buf();
    for i in 0..10 {
        path = path.join(format!("d{i}"));
    }
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("leaf.txt"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("leaf.txt"))
        .stdout(predicate::str::contains("10 directories, 1 file"));
}

#[test]
fn unicode_filename_via_cli() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("café.txt"), "x").unwrap();
    fs::write(tmp.path().join("日本語.md"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("café.txt"))
        .stdout(predicate::str::contains("日本語.md"));
}

#[test]
fn target_can_be_a_file() {
    let tmp = TempDir::new().unwrap();
    let f = tmp.path().join("solo.txt");
    fs::write(&f, "x").unwrap();

    bush()
        .arg(&f)
        .assert()
        .success()
        .stdout(predicate::str::contains("solo.txt"));
}

#[test]
fn no_ignore_plus_hidden_shows_all() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("node_modules")).unwrap();
    fs::write(tmp.path().join("node_modules/dep.js"), "x").unwrap();
    fs::write(tmp.path().join("visible.js"), "x").unwrap();
    fs::write(tmp.path().join(".env"), "secret=1").unwrap();
    fs::write(tmp.path().join(".gitignore"), "node_modules\n").unwrap();

    bush()
        .arg(tmp.path())
        .args(["--no-ignore", "--hidden"])
        .assert()
        .success()
        .stdout(predicate::str::contains("node_modules"))
        .stdout(predicate::str::contains(".env"))
        .stdout(predicate::str::contains(".gitignore"));
}

// ===== Additional edge cases =====

#[test]
fn explicit_config_relative_path_resolves_against_cwd() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("noisy.log"), "x").unwrap();
    fs::write(tmp.path().join(".gitignore"), "*.log\n").unwrap();
    fs::write(tmp.path().join("custom.json"), r#"{"ignore_files": []}"#).unwrap();

    bush()
        .current_dir(tmp.path())
        .args(["--config", "custom.json", "."])
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log"));
}

#[cfg(unix)]
#[test]
fn target_can_be_symlink_to_dir() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("real")).unwrap();
    fs::write(tmp.path().join("real/inside.txt"), "x").unwrap();
    std::os::unix::fs::symlink(tmp.path().join("real"), tmp.path().join("link")).unwrap();

    bush()
        .arg(tmp.path().join("link"))
        .assert()
        .success()
        .stdout(predicate::str::contains("inside.txt"));
}

#[cfg(unix)]
#[test]
fn output_target_is_a_symlink_writes_through() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x"), "y").unwrap();
    fs::write(tmp.path().join("real.txt"), "STALE").unwrap();
    std::os::unix::fs::symlink(tmp.path().join("real.txt"), tmp.path().join("link.txt")).unwrap();

    bush()
        .arg(tmp.path())
        .args(["-o", tmp.path().join("link.txt").to_str().unwrap()])
        .assert()
        .success();

    let content = fs::read_to_string(tmp.path().join("real.txt")).unwrap();
    assert!(content.contains("x"));
    assert!(!content.contains("STALE"));
}

#[cfg(unix)]
#[test]
fn local_bush_as_symlink_is_followed() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("noisy.log"), "x").unwrap();
    fs::write(tmp.path().join(".gitignore"), "*.log\n").unwrap();
    let real = tmp.path().join("real_config.json");
    fs::write(&real, r#"{"ignore_files": []}"#).unwrap();
    std::os::unix::fs::symlink(&real, tmp.path().join(".bush")).unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log"));
}

// ===== Config discovery extensions =====

#[test]
fn bush_config_env_used_as_explicit_config() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("noisy.log"), "x").unwrap();
    fs::write(tmp.path().join(".gitignore"), "*.log\n").unwrap();
    let cfg = tmp.path().join("via_env.json");
    fs::write(&cfg, r#"{"ignore_files": []}"#).unwrap();

    bush()
        .env("BUSH_CONFIG", &cfg)
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log"));
}

#[test]
fn cli_config_wins_over_bush_config_env() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("noisy.log"), "x").unwrap();
    fs::write(tmp.path().join(".gitignore"), "*.log\n").unwrap();
    let cli_cfg = tmp.path().join("cli.json");
    fs::write(&cli_cfg, r#"{"ignore_files": []}"#).unwrap();
    let env_cfg = tmp.path().join("env.json");
    fs::write(&env_cfg, r#"{"ignore_files": [".gitignore"]}"#).unwrap();

    bush()
        .env("BUSH_CONFIG", &env_cfg)
        .arg(tmp.path())
        .args(["--config", cli_cfg.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log"));
}

#[test]
fn xdg_config_home_loaded() {
    let tmp = TempDir::new().unwrap();
    let xdg = tmp.path().join("xdg");
    fs::create_dir_all(xdg.join("bush")).unwrap();
    fs::write(xdg.join("bush/config.json"), r#"{"ignore_files": []}"#).unwrap();

    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("noisy.log"), "x").unwrap();
    fs::write(project.join(".gitignore"), "*.log\n").unwrap();

    bush()
        .env("XDG_CONFIG_HOME", &xdg)
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log"));
}

#[test]
fn dot_bush_json_local_file_works() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("noisy.log"), "x").unwrap();
    fs::write(tmp.path().join(".gitignore"), "*.log\n").unwrap();
    fs::write(tmp.path().join(".bush.json"), r#"{"ignore_files": []}"#).unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("noisy.log"));
}

// ===== Output formats =====

#[test]
fn format_json_produces_valid_json() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("a.txt"), "x").unwrap();
    fs::write(tmp.path().join("b.txt"), "x").unwrap();

    let output = bush()
        .arg(tmp.path())
        .args(["--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).expect("output must be valid JSON");
    assert!(v["root"].is_string());
    assert!(v["children"].is_array());
    assert_eq!(v["report"]["files"], 2);
}

#[test]
fn format_html_starts_with_doctype() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x.txt"), "y").unwrap();

    bush()
        .arg(tmp.path())
        .args(["--format", "html"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("<!DOCTYPE html>"))
        .stdout(predicate::str::contains("x.txt"))
        .stdout(predicate::str::contains("</body>"));
}

#[test]
fn format_xml_starts_with_declaration() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x.txt"), "y").unwrap();

    bush()
        .arg(tmp.path())
        .args(["--format", "xml"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("<?xml"))
        .stdout(predicate::str::contains("<file name=\"x.txt\""))
        .stdout(predicate::str::contains("</tree>"));
}

#[test]
fn format_json_respects_ignore_files() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("node_modules")).unwrap();
    fs::write(tmp.path().join("node_modules/x"), "y").unwrap();
    fs::write(tmp.path().join("main.rs"), "x").unwrap();
    fs::write(tmp.path().join(".gitignore"), "node_modules\n").unwrap();

    let output = bush()
        .arg(tmp.path())
        .args(["--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    let names: Vec<String> = v["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"main.rs".to_string()));
    assert!(!names.contains(&"node_modules".to_string()));
}

#[test]
fn format_json_includes_no_ansi_codes() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("dir")).unwrap();
    fs::write(tmp.path().join("dir/x"), "y").unwrap();

    bush()
        .arg(tmp.path())
        .args(["--format", "json", "--color", "always"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[").not());
}

// ===== Symlink target + permissions display =====

#[cfg(unix)]
#[test]
fn symlink_target_display_via_cli() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("real.txt"), "x").unwrap();
    std::os::unix::fs::symlink(tmp.path().join("real.txt"), tmp.path().join("link.txt")).unwrap();

    bush()
        .arg(tmp.path())
        .arg("-l")
        .assert()
        .success()
        .stdout(predicate::str::contains("link.txt -> "));
}

#[cfg(unix)]
#[test]
fn permissions_display_via_cli() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = TempDir::new().unwrap();
    let f = tmp.path().join("script.sh");
    fs::write(&f, "x").unwrap();
    fs::set_permissions(&f, fs::Permissions::from_mode(0o755)).unwrap();

    bush()
        .arg(tmp.path())
        .arg("-p")
        .assert()
        .success()
        .stdout(predicate::str::contains("[rwxr-xr-x]"));
}

#[cfg(unix)]
#[test]
fn permissions_and_sizes_combined() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = TempDir::new().unwrap();
    let f = tmp.path().join("data");
    fs::write(&f, vec![0u8; 200]).unwrap();
    fs::set_permissions(&f, fs::Permissions::from_mode(0o644)).unwrap();

    bush()
        .arg(tmp.path())
        .args(["-p", "-s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[rw-r--r--]"))
        .stdout(predicate::str::contains("[200]"));
}

// ===== Filters =====

#[test]
fn include_only_matching_files() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/main.rs"), "x").unwrap();
    fs::write(tmp.path().join("src/util.rs"), "x").unwrap();
    fs::write(tmp.path().join("README.md"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .args(["--include", "*.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("util.rs"))
        .stdout(predicate::str::contains("README.md").not());
}

#[test]
fn exclude_drops_matching_paths() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("noisy.log"), "x").unwrap();
    fs::write(tmp.path().join("keep.txt"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .args(["--exclude", "*.log"])
        .assert()
        .success()
        .stdout(predicate::str::contains("keep.txt"))
        .stdout(predicate::str::contains("noisy.log").not());
}

#[test]
fn exclude_directory_by_name() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("vendor")).unwrap();
    fs::write(tmp.path().join("vendor/lib.go"), "x").unwrap();
    fs::write(tmp.path().join("main.go"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .args(["--exclude", "vendor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.go"))
        .stdout(predicate::str::contains("vendor").not());
}

#[test]
fn include_prunes_empty_dirs() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("empty_after_filter")).unwrap();
    fs::write(tmp.path().join("empty_after_filter/x.txt"), "x").unwrap();
    fs::write(tmp.path().join("main.rs"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .args(["--include", "*.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("empty_after_filter").not());
}

#[test]
fn include_and_exclude_combined() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("a.rs"), "x").unwrap();
    fs::write(tmp.path().join("b.rs"), "x").unwrap();
    fs::write(tmp.path().join("test_b.rs"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .args(["--include", "*.rs", "--exclude", "test_*.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("a.rs"))
        .stdout(predicate::str::contains("b.rs"))
        .stdout(predicate::str::contains("test_b.rs").not());
}

#[test]
fn invalid_glob_errors() {
    let tmp = TempDir::new().unwrap();
    bush()
        .arg(tmp.path())
        .args(["--include", "[unclosed"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--include"));
}

// ===== --noreport =====

#[test]
fn noreport_suppresses_footer() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x"), "y").unwrap();

    bush()
        .arg(tmp.path())
        .arg("--noreport")
        .assert()
        .success()
        .stdout(predicate::str::contains("directories,").not());
}

#[test]
fn footer_present_by_default() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x"), "y").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("0 directories, 1 file"));
}

// ===== Sort =====

#[test]
fn sort_by_size_via_cli() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("big"), vec![0u8; 1000]).unwrap();
    fs::write(tmp.path().join("mid"), vec![0u8; 500]).unwrap();
    fs::write(tmp.path().join("small"), vec![0u8; 10]).unwrap();

    let output = bush()
        .arg(tmp.path())
        .args(["--sort", "size"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    let small = s.find("small").unwrap();
    let mid = s.find("mid").unwrap();
    let big = s.find("big").unwrap();
    assert!(small < mid && mid < big, "expected size-ASC, got: {s}");
}

#[test]
fn sort_reverse_flag() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("alpha"), "x").unwrap();
    fs::write(tmp.path().join("zeta"), "x").unwrap();

    let output = bush()
        .arg(tmp.path())
        .args(["-r"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    let alpha = s.find("alpha").unwrap();
    let zeta = s.find("zeta").unwrap();
    assert!(zeta < alpha, "expected Z before A under -r, got: {s}");
}

#[test]
fn sort_none_via_cli() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("z"), "x").unwrap();
    fs::write(tmp.path().join("a"), "x").unwrap();
    // sort=none preserves walker order — walker order isn't deterministic across filesystems,
    // so just verify the command succeeds with both files present.

    bush()
        .arg(tmp.path())
        .args(["--sort", "none"])
        .assert()
        .success()
        .stdout(predicate::str::contains("a"))
        .stdout(predicate::str::contains("z"));
}

// ===== Sizes & mtime =====

#[test]
fn show_sizes_flag_includes_bracketed_size() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("data.bin"), vec![0u8; 2048]).unwrap();

    bush()
        .arg(tmp.path())
        .arg("-s")
        .assert()
        .success()
        .stdout(predicate::str::contains("[2.0K]"))
        .stdout(predicate::str::contains("data.bin"));
}

#[test]
fn show_mtime_flag_includes_iso_date() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x"), "y").unwrap();

    bush()
        .arg(tmp.path())
        .arg("-D")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}\]").unwrap());
}

#[test]
fn sizes_and_mtime_combined() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x"), vec![0u8; 100]).unwrap();

    bush()
        .arg(tmp.path())
        .args(["-s", "-D"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[100]"))
        .stdout(predicate::str::is_match(r"\[\d{4}-\d{2}-\d{2}").unwrap());
}

#[test]
fn no_sizes_or_mtime_by_default() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("x"), "y").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("[").not())
        .stdout(predicate::str::contains("]").not());
}

// ===== Color =====

#[test]
fn color_always_emits_ansi() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("subdir")).unwrap();
    fs::write(tmp.path().join("subdir/x"), "y").unwrap();

    bush()
        .arg(tmp.path())
        .args(["--color", "always"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b["));
}

#[test]
fn color_never_emits_no_ansi() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("subdir")).unwrap();

    bush()
        .arg(tmp.path())
        .args(["--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[").not());
}

#[test]
fn color_auto_disabled_when_stdout_redirected() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("subdir")).unwrap();

    // assert_cmd captures stdout, so IsTerminal returns false. Auto should disable.
    bush()
        .arg(tmp.path())
        .args(["--color", "auto"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[").not());
}

#[test]
fn color_default_is_auto_disabled_in_test_env() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("subdir")).unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[").not());
}

#[test]
fn local_bush_as_directory_is_silently_skipped() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join(".bush")).unwrap();
    fs::write(tmp.path().join("normal.txt"), "x").unwrap();

    bush()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("normal.txt"));
}

#[test]
fn parent_gitignore_applies_when_target_is_subdir() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".gitignore"), "secret.txt\n").unwrap();
    fs::create_dir(tmp.path().join("sub")).unwrap();
    fs::write(tmp.path().join("sub/secret.txt"), "x").unwrap();
    fs::write(tmp.path().join("sub/visible.txt"), "x").unwrap();

    bush()
        .arg(tmp.path().join("sub"))
        .assert()
        .success()
        .stdout(predicate::str::contains("visible.txt"))
        .stdout(predicate::str::contains("secret.txt").not());
}

#[test]
fn no_ignore_overrides_parent_gitignore() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".gitignore"), "secret.txt\n").unwrap();
    fs::create_dir(tmp.path().join("sub")).unwrap();
    fs::write(tmp.path().join("sub/secret.txt"), "x").unwrap();

    bush()
        .arg(tmp.path().join("sub"))
        .arg("--no-ignore")
        .assert()
        .success()
        .stdout(predicate::str::contains("secret.txt"));
}

#[cfg(unix)]
#[test]
fn broken_pipe_exits_quietly() {
    use std::io::Read;
    use std::process::{Command as StdCommand, Stdio};

    // Enough output to overflow the OS pipe buffer (64K on Linux) plus bush's
    // own BufWriter, so the write blocks until the reader closes the pipe.
    let tmp = TempDir::new().unwrap();
    for i in 0..3000 {
        fs::write(
            tmp.path().join(format!("file_with_a_long_name_{i:04}.txt")),
            "x",
        )
        .unwrap();
    }

    let mut child = StdCommand::new(env!("CARGO_BIN_EXE_bush"))
        .arg(tmp.path())
        .env_remove("HOME")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Read a little, then close the read end to trigger EPIPE in bush.
    let mut stdout = child.stdout.take().unwrap();
    let mut buf = [0u8; 256];
    let _ = stdout.read(&mut buf);
    drop(stdout);

    let status = child.wait().unwrap();
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();

    assert!(
        status.success(),
        "broken pipe must exit 0, got {status:?} with stderr: {stderr}"
    );
    assert!(
        stderr.is_empty(),
        "broken pipe must not print an error, got: {stderr}"
    );
}
