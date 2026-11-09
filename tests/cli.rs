use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
    process::Command,
};

fn invoke(config: &Path, args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_markdown-todo"))
        .env("XDG_CONFIG_HOME", config)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn config(directory: &Path, path: &Path) {
    fs::create_dir_all(directory.join("markdown-todo")).unwrap();
    fs::write(
        directory.join("markdown-todo/config.json"),
        json!({"path": path}).to_string(),
    )
    .unwrap();
}

#[test]
fn configured_file_symlink_and_external_changes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("tasks with ' quotes.md");
    let link = directory.path().join("link.md");
    fs::write(&path, "- [ ] task\n").unwrap();
    symlink(&path, &link).unwrap();
    config(directory.path(), &link);
    let first = invoke(directory.path(), &["load"]);
    let revision = first["revision"].as_str().unwrap();
    let updated = invoke(directory.path(), &["toggle", revision, "3"]);
    assert!(updated.get("error").is_none());
    assert_eq!(fs::read_to_string(&path).unwrap(), "- [x] task\n");
    assert!(link.is_symlink());
    fs::write(&path, "- [ ] new external text\n").unwrap();
    let stale = invoke(directory.path(), &["toggle", revision, "3"]);
    assert!(
        stale["error"]
            .as_str()
            .unwrap()
            .contains("changed elsewhere")
    );
    assert!(
        stale["html"]
            .as_str()
            .unwrap()
            .contains("new external text")
    );
}

#[test]
fn configuration_and_read_errors_are_json() {
    let directory = tempfile::tempdir().unwrap();
    config(directory.path(), Path::new("relative.md"));
    assert!(
        invoke(directory.path(), &["load"])["error"]
            .as_str()
            .unwrap()
            .contains("absolute")
    );
    config(directory.path(), &directory.path().join("missing.md"));
    assert!(
        invoke(directory.path(), &["load"])["error"]
            .as_str()
            .unwrap()
            .contains("Cannot read")
    );
    fs::write(directory.path().join("markdown-todo/config.json"), "{").unwrap();
    assert!(
        invoke(directory.path(), &["load"])["error"]
            .as_str()
            .unwrap()
            .contains("Invalid")
    );
}

#[test]
fn editor_arguments_and_path_are_passed_without_shell_evaluation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("tasks ' with spaces.md");
    config(directory.path(), &path);
    let terminal = directory.path().join("konsole");
    let editor = directory.path().join("fake-editor");
    let log = directory.path().join("arguments");
    fs::write(
        &terminal,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$TODO_TEST_LOG\"\n",
    )
    .unwrap();
    fs::write(&editor, "#!/bin/sh\nexit 0\n").unwrap();
    for script in [&terminal, &editor] {
        fs::set_permissions(script, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let run = |editor_setting: &str| {
        fs::write(
            directory.path().join("markdown-todo/config.json"),
            json!({
                "path": path, "editor": editor_setting, "is_graphical": false,
                "terminal": terminal,
            })
            .to_string(),
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_markdown-todo"))
            .env("XDG_CONFIG_HOME", directory.path())
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", directory.path().display()),
            )
            .env_remove("VISUAL")
            .env("TODO_TEST_LOG", &log)
            .arg("edit")
            .output()
            .unwrap();
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    assert_eq!(run("fake-editor --flag 'two words'")["editing"], true);
    assert_eq!(
        fs::read_to_string(&log).unwrap(),
        format!(
            "--separate\n-e\nfake-editor\n--flag\ntwo words\n{}\n",
            path.display()
        )
    );
    assert!(run("")["error"].as_str().unwrap().contains("VISUAL"));
    assert!(
        run("does-not-exist")["error"]
            .as_str()
            .unwrap()
            .contains("not found")
    );
}

#[test]
fn graphical_editor_and_visual_fallback() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("tasks with spaces.md");
    config(directory.path(), &path);
    let primary = directory.path().join("primary");
    let visual = directory.path().join("visual");
    let failing = directory.path().join("failing");
    let log = directory.path().join("launch");
    for script in [&primary, &visual] {
        fs::write(
            script,
            "#!/bin/sh\nprintf '%s\\n' \"$0\" \"$@\" > \"$TODO_TEST_LOG\"\n",
        )
        .unwrap();
        fs::set_permissions(script, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(&failing, "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(&failing, fs::Permissions::from_mode(0o755)).unwrap();
    let run = |editor: Option<&Path>, graphical: bool, terminal: &Path| {
        fs::write(
            directory.path().join("markdown-todo/config.json"),
            json!({
                "path": path, "editor": editor, "is_graphical": graphical, "terminal": terminal,
            })
            .to_string(),
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_markdown-todo"))
            .env("XDG_CONFIG_HOME", directory.path())
            .env(
                "VISUAL",
                format!("'{}' --flag 'two words'", visual.display()),
            )
            .env("TODO_TEST_LOG", &log)
            .arg("edit")
            .output()
            .unwrap();
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let missing_terminal = directory.path().join("missing-terminal");
    assert_eq!(
        run(Some(&primary), true, &missing_terminal)["editing"],
        true
    );
    assert_eq!(
        fs::read_to_string(&log).unwrap(),
        format!("{}\n{}\n", primary.display(), path.display())
    );
    for (editor, graphical, terminal) in [
        (None, true, &missing_terminal),
        (Some(Path::new("/does-not-exist")), true, &missing_terminal),
        (Some(failing.as_path()), true, &missing_terminal),
        (Some(primary.as_path()), false, &missing_terminal),
        (Some(primary.as_path()), false, &failing),
    ] {
        assert_eq!(run(editor, graphical, terminal)["editing"], true);
        assert_eq!(
            fs::read_to_string(&log).unwrap(),
            format!(
                "{}\n--flag\ntwo words\n{}\n",
                visual.display(),
                path.display()
            )
        );
    }
    let output = Command::new(env!("CARGO_BIN_EXE_markdown-todo"))
        .env("XDG_CONFIG_HOME", directory.path())
        .env("VISUAL", "/does-not-exist")
        .arg("edit")
        .output()
        .unwrap();
    let error: Value = serde_json::from_slice(&output.stdout).unwrap();
    let message = error["error"].as_str().unwrap();
    assert!(
        message.contains("Configured editor failed") && message.contains("$VISUAL fallback failed")
    );
}
