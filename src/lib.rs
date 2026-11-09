use anyhow::{Context, Result, bail};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs, io::Write, path::Path};
use tempfile::NamedTempFile;

#[derive(Default, Serialize)]
pub struct Snapshot {
    pub path: String,
    pub revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    pub unchanged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn revision(path: &Path, bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(path.as_os_str().as_encoded_bytes());
    hash.update([0]);
    hash.update(bytes);
    format!("{:x}", hash.finalize())
}

fn parser(source: &str) -> Parser<'_> {
    Parser::new_ext(source, Options::ENABLE_TASKLISTS)
}

pub fn render(source: &str, version: &str) -> String {
    let parsed: Vec<_> = parser(source).into_offset_iter().collect();
    let mut items = Vec::new();
    let mut tasks = HashMap::new();
    for (index, (event, range)) in parsed.iter().enumerate() {
        match event {
            Event::Start(Tag::Item) => items.push(index),
            Event::End(TagEnd::Item) => {
                items.pop();
            }
            Event::TaskListMarker(checked) => {
                if let Some(item) = items.last() {
                    tasks.insert(*item, (*checked, range.start + 1));
                }
            }
            _ => {}
        }
    }
    let mut checked_items = Vec::new();
    let mut lists = Vec::new();
    let mut events = Vec::new();
    for (index, (event, _)) in parsed.into_iter().enumerate() {
        let event = match event {
            // Keep user HTML inert before adding our own markup.
            Event::Html(text) | Event::InlineHtml(text) => Event::Text(text),
            // Qt rich text ignores list-style:none. Cells keep a single marker
            // aligned with wrapped text and preserve nested list indentation.
            Event::Start(Tag::List(start)) => {
                lists.push(start);
                Event::Html("<table width=\"100%\" cellspacing=\"0\" cellpadding=\"2\">".into())
            }
            Event::End(TagEnd::List(_)) => {
                lists.pop();
                Event::Html("</table>".into())
            }
            Event::Start(Tag::Item) => {
                let task = tasks.get(&index);
                checked_items.push(task.is_some_and(|(checked, _)| *checked));
                let marker = if let Some((checked, offset)) = task {
                    let (symbol, label) = if *checked {
                        ("☑", "Uncheck task")
                    } else {
                        ("☐", "Check task")
                    };
                    format!(
                        "<a href=\"todo:{version}:{offset}\" title=\"{label}\" \
                        style=\"color:#b8b8b8; font-size:16px; text-decoration:none;\">{symbol}</a>"
                    )
                } else if let Some(Some(number)) = lists.last() {
                    format!("{number}.")
                } else {
                    "•".into()
                };
                if let Some(Some(number)) = lists.last_mut() {
                    *number += 1;
                }
                Event::Html(
                    format!("<tr><td width=\"22\" valign=\"top\">{marker}</td><td valign=\"top\">")
                        .into(),
                )
            }
            Event::End(TagEnd::Item) => {
                checked_items.pop();
                Event::Html("</td></tr>".into())
            }
            other => other,
        };
        let completed_text =
            checked_items.last() == Some(&true) && matches!(event, Event::Text(_) | Event::Code(_));
        if completed_text {
            events.push(Event::InlineHtml(
                "<s><span style=\"color:#858585;\">".into(),
            ));
        }
        let event = match event {
            Event::TaskListMarker(_) => continue,
            // Omit images. Their alt text still renders.
            Event::Start(Tag::Image { .. }) | Event::End(TagEnd::Image) => continue,
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                let safe = ["https://", "http://", "mailto:", "#"]
                    .iter()
                    .any(|prefix| dest_url.starts_with(prefix));
                Event::Start(Tag::Link {
                    link_type,
                    dest_url: if safe { dest_url } else { "".into() },
                    title,
                    id,
                })
            }
            other => other,
        };
        events.push(event);
        if completed_text {
            events.push(Event::InlineHtml("</span></s>".into()));
        }
    }
    let mut output = String::from(
        "<style>\
        a { color:#b8b8b8; } \
        p { margin-top:6px; margin-bottom:6px; } \
        </style>",
    );
    html::push_html(&mut output, events.into_iter());
    output
}

pub fn load(path: &Path, known: &str) -> Result<Snapshot> {
    let bytes = fs::read(path).with_context(|| format!("Cannot read {}", path.display()))?;
    snapshot(path, &bytes, known)
}

fn snapshot(path: &Path, bytes: &[u8], known: &str) -> Result<Snapshot> {
    let source = std::str::from_utf8(bytes).context("The todo file must be UTF-8")?;
    let version = revision(path, bytes);
    let unchanged = version == known;
    let rendered = (!unchanged).then(|| render(source, &version));
    Ok(Snapshot {
        path: path.display().to_string(),
        revision: version,
        html: rendered,
        unchanged,
        error: None,
    })
}

fn stale(path: &Path) -> Result<Snapshot> {
    let mut state = load(path, "")?;
    state.error = Some("File changed elsewhere. Reloaded; click the checkbox again.".into());
    Ok(state)
}

pub fn toggle(path: &Path, expected: &str, offset: usize) -> Result<Snapshot> {
    let bytes = fs::read(path).with_context(|| format!("Cannot read {}", path.display()))?;
    if revision(path, &bytes) != expected {
        return stale(path);
    }
    let source = std::str::from_utf8(&bytes).context("The todo file must be UTF-8")?;
    let valid = parser(source).into_offset_iter().any(|(event, range)| {
        matches!(event, Event::TaskListMarker(_)) && range.start + 1 == offset
    });
    if !valid || !matches!(bytes.get(offset), Some(b' ' | b'x' | b'X')) {
        bail!("This checkbox is no longer available. Reload and try again.");
    }
    // Opening for writing enforces file permissions even though replacement uses the directory.
    let original = fs::OpenOptions::new()
        .write(true)
        .open(path)
        .with_context(|| format!("Cannot write {}", path.display()))?;
    let permissions = original.metadata()?.permissions();
    let mut updated = bytes.clone();
    updated[offset] = if updated[offset] == b' ' { b'x' } else { b' ' };
    let mut temp = NamedTempFile::new_in(path.parent().context("File has no parent directory")?)?;
    temp.as_file().set_permissions(permissions)?;
    temp.write_all(&updated)?;
    temp.as_file().sync_all()?;
    // Catch external saves made while the replacement was being prepared.
    if fs::read(path)? != bytes {
        return stale(path);
    }
    temp.persist(path)
        .with_context(|| format!("Cannot save {}", path.display()))?;
    snapshot(path, &updated, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_markdown_and_only_real_tasks() {
        let source = "# Today\n\n**bold** *italic* [link](https://kde.org)\n\n\
                      - [ ] parent\n  - [X] child\n\n```md\n- [ ] example\n```\n\n\
                      <img src=\"file:///secret\">\n\n![alt](https://invalid/image.png)\n";
        let output = render(source, "version");
        assert!(output.contains("<h1>Today</h1>"));
        assert!(output.contains("<strong>bold</strong>"));
        assert!(output.contains("<em>italic</em>"));
        assert!(output.contains("href=\"https://kde.org\""));
        assert_eq!(output.matches("href=\"todo:version:").count(), 2);
        assert!(output.contains("☐") && output.contains("☑"));
        assert!(output.contains("<pre><code"));
        assert!(!output.contains("<img"));
        assert!(output.contains("&lt;img"));
        assert!(output.contains("alt"));
    }

    #[test]
    fn completed_text_is_struck_without_affecting_children_or_checkbox_links() {
        let output = render(
            "- [x] done **bold**\n  - [ ] child\n- [ ] next\n",
            "version",
        );
        assert!(output.contains("<s><span style=\"color:#858585;\">done "));
        assert!(output.contains("<strong><s><span style=\"color:#858585;\">bold"));
        assert!(!output.contains("<li"));
        assert!(!output.contains("•"));
        assert!(output.contains("<td valign=\"top\">child"));
        assert!(output.contains("<td valign=\"top\">next"));
        assert_eq!(output.matches("title=\"Check task\"").count(), 2);
        assert_eq!(output.matches("title=\"Uncheck task\"").count(), 1);
    }

    #[test]
    fn toggle_preserves_every_other_byte_and_parent_state() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("tasks.md");
        let source = "# café\r\n- [ ] parent\r\n  - [X] child\r\n";
        fs::write(&path, source)?;
        let offset = source.find("[X]").unwrap() + 1;
        let before = load(&path, "")?;
        let after = toggle(&path, &before.revision, offset)?;
        assert_eq!(fs::read_to_string(&path)?, source.replace("[X]", "[ ]"));
        assert_ne!(before.revision, after.revision);
        toggle(&path, &after.revision, offset)?;
        assert_eq!(fs::read_to_string(&path)?, source.replace("[X]", "[x]"));
        Ok(())
    }

    #[test]
    fn stale_click_reloads_without_writing() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("tasks.md");
        fs::write(&path, "- [ ] old\n")?;
        let before = load(&path, "")?;
        let external = "- [ ] externally edited\n";
        fs::write(&path, external)?;
        let after = toggle(&path, &before.revision, 3)?;
        assert!(after.error.is_some());
        assert!(after.html.unwrap().contains("externally edited"));
        assert_eq!(fs::read_to_string(&path)?, external);
        Ok(())
    }

    #[test]
    fn code_examples_cannot_be_toggled() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("tasks.md");
        let source = "```\n- [ ] fake\n```\n";
        fs::write(&path, source)?;
        let before = load(&path, "")?;
        assert!(toggle(&path, &before.revision, source.find("[ ]").unwrap() + 1).is_err());
        assert_eq!(fs::read_to_string(&path)?, source);
        Ok(())
    }

    #[test]
    fn unchanged_load_skips_rendering_and_path_changes_invalidate_revision() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("tasks.md");
        fs::write(&path, "- [ ] task\n")?;
        let first = load(&path, "")?;
        let second = load(&path, &first.revision)?;
        assert!(second.unchanged && second.html.is_none());
        assert_ne!(
            revision(&path, b"same"),
            revision(&path.with_extension("txt"), b"same")
        );
        Ok(())
    }

    #[test]
    fn read_only_file_does_not_change() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("tasks.md");
        let source = "- [ ] task\n";
        fs::write(&path, source)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444))?;
        let before = load(&path, "")?;
        assert!(toggle(&path, &before.revision, 3).is_err());
        assert_eq!(fs::read_to_string(&path)?, source);
        Ok(())
    }
}
