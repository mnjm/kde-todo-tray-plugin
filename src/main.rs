use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

#[derive(Deserialize)]
#[serde(default)]
struct Config {
    path: Option<String>,
    editor: Option<String>,
    is_graphical: bool,
    terminal: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            path: None,
            editor: None,
            is_graphical: true,
            terminal: "konsole".into(),
        }
    }
}

fn read_config() -> Result<Config> {
    let home = PathBuf::from(env::var_os("HOME").context("HOME is not set")?);
    let config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".config"));
    let config_path = config_home.join("markdown-todo/config.json");
    match fs::read_to_string(&config_path) {
        Ok(text) => serde_json::from_str(&text).context("Invalid markdown-todo/config.json"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(error) => Err(error).context("Cannot read markdown-todo/config.json"),
    }
}

fn todo_path(config: &Config) -> Result<PathBuf> {
    let home = PathBuf::from(env::var_os("HOME").context("HOME is not set")?);
    let raw = config.path.as_deref().unwrap_or("~/todo.md");
    let path = raw
        .strip_prefix("~/")
        .map(|suffix| home.join(suffix))
        .unwrap_or_else(|| PathBuf::from(raw));
    if !path.is_absolute() {
        bail!("The todo path must be absolute or start with ~/.");
    }
    // Follow symlinks so saving never replaces the user's symlink itself.
    if path.exists() {
        Ok(path.canonicalize()?)
    } else {
        Ok(path)
    }
}

fn executable_exists(name: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let executable = |path: &Path| {
        path.is_file()
            && path
                .metadata()
                .is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
    };
    if name.contains('/') {
        return executable(Path::new(name));
    }
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| executable(&directory.join(name)))
    })
}

fn launch_editor(path: &Path, setting: &str, is_graphical: bool, terminal: &str) -> Result<()> {
    let editor = shell_words::split(setting).context("Cannot parse editor command")?;
    let program = editor.first().context("Editor command is empty")?;
    if !executable_exists(program) {
        bail!("Editor not found: {program}");
    }
    let mut command = if is_graphical {
        let mut command = Command::new(program);
        command.args(&editor[1..]);
        command
    } else {
        let mut command = Command::new(terminal);
        if Path::new(terminal)
            .file_name()
            .is_some_and(|name| name == "konsole")
        {
            command.arg("--separate");
        }
        command.arg("-e").args(&editor);
        command
    };
    let mut child = command
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Cannot launch editor or terminal")?;
    thread::sleep(Duration::from_millis(200));
    if child.try_wait()?.is_some_and(|status| !status.success()) {
        bail!("Editor or terminal exited with an error.");
    }
    Ok(())
}

fn edit(path: &Path, config: &Config) -> Result<Value> {
    let primary_error = if let Some(editor) = config
        .editor
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        match launch_editor(path, editor, config.is_graphical, &config.terminal) {
            Ok(()) => return Ok(json!({"editing": true})),
            Err(error) => Some(error),
        }
    } else {
        None
    };
    let fallback = env::var("VISUAL").unwrap_or_default();
    if fallback.trim().is_empty() {
        if let Some(error) = primary_error {
            return Err(error).context("Configured editor failed and $VISUAL is unset");
        }
        bail!("Configure editor or set $VISUAL in the Plasma session environment.");
    }
    launch_editor(path, &fallback, true, &config.terminal).map_err(|error| {
        if let Some(primary) = primary_error {
            error.context(format!(
                "Configured editor failed ({primary:#}); $VISUAL fallback failed"
            ))
        } else {
            error.context("$VISUAL fallback failed")
        }
    })?;
    Ok(json!({"editing": true}))
}

fn run() -> Result<Value> {
    let config = read_config()?;
    let path = todo_path(&config)?;
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("load") => Ok(serde_json::to_value(markdown_todo::load(
            &path,
            args.get(2).map_or("", String::as_str),
        )?)?),
        Some("toggle") => Ok(serde_json::to_value(markdown_todo::toggle(
            &path,
            args.get(2).context("Missing revision")?,
            args.get(3).context("Missing checkbox offset")?.parse()?,
        )?)?),
        Some("edit") => edit(&path, &config),
        _ => bail!("Usage: markdown-todo load [revision] | toggle <revision> <offset> | edit"),
    }
}

fn main() {
    let response = run().unwrap_or_else(|error| json!({"error": format!("{error:#}")}));
    println!("{response}");
}
