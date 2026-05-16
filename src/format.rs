//! JSON, HTML, and XML output renderers.

use crate::format_meta;
use crate::tree::{counts, Node, NodeMeta};
use anyhow::Result;
use std::io::Write;
use std::path::Path;

// ===== JSON =====

pub fn render_json<W: Write>(root: &Path, tree: &Node, out: &mut W) -> Result<()> {
    let value = node_to_json(Some(root), tree);
    let pretty = serde_json::to_string_pretty(&value)?;
    out.write_all(pretty.as_bytes())?;
    out.write_all(b"\n")?;
    Ok(())
}

fn node_to_json(root: Option<&Path>, node: &Node) -> serde_json::Value {
    use serde_json::{Map, Value};
    let mut obj: Map<String, Value> = Map::new();
    if let Some(r) = root {
        obj.insert("root".into(), Value::String(r.display().to_string()));
    } else {
        obj.insert("name".into(), Value::String(node.name.clone()));
    }
    obj.insert("type".into(), Value::String(node_type(&node.meta).into()));
    if !node.meta.is_dir {
        obj.insert(
            "size".into(),
            Value::Number(serde_json::Number::from(node.meta.size)),
        );
    }
    if let Some(t) = node.meta.mtime {
        obj.insert("mtime".into(), Value::String(format_meta::format_mtime(t)));
    }
    if let Some(m) = node.meta.mode {
        obj.insert("mode".into(), Value::String(format_meta::format_mode(m)));
    }
    if let Some(target) = &node.meta.symlink_target {
        obj.insert(
            "symlink_target".into(),
            Value::String(target.display().to_string()),
        );
    }
    if !node.children.is_empty() {
        let children: Vec<Value> = node
            .children
            .iter()
            .map(|c| node_to_json(None, c))
            .collect();
        obj.insert("children".into(), Value::Array(children));
    }
    if root.is_some() {
        let (dirs, files) = counts(node);
        let mut report = Map::new();
        report.insert(
            "directories".into(),
            Value::Number(serde_json::Number::from(dirs)),
        );
        report.insert(
            "files".into(),
            Value::Number(serde_json::Number::from(files)),
        );
        obj.insert("report".into(), Value::Object(report));
    }
    Value::Object(obj)
}

fn node_type(meta: &NodeMeta) -> &'static str {
    if meta.is_symlink {
        "symlink"
    } else if meta.is_dir {
        "directory"
    } else {
        "file"
    }
}

// ===== HTML =====

pub fn render_html<W: Write>(root: &Path, tree: &Node, out: &mut W) -> Result<()> {
    writeln!(out, "<!DOCTYPE html>")?;
    writeln!(out, "<html lang=\"en\">")?;
    writeln!(out, "<head>")?;
    writeln!(out, "  <meta charset=\"UTF-8\">")?;
    writeln!(
        out,
        "  <title>bush: {}</title>",
        html_escape(&root.display().to_string())
    )?;
    writeln!(out, "  <style>")?;
    writeln!(out, "    body {{ font-family: monospace; }}")?;
    writeln!(
        out,
        "    ul.bush-tree {{ list-style: none; padding-left: 1.2em; }}"
    )?;
    writeln!(out, "    li.directory > span.name {{ font-weight: bold; }}")?;
    writeln!(out, "    li.symlink > span.target {{ color: #888; }}")?;
    writeln!(out, "    p.bush-report {{ margin-top: 1em; }}")?;
    writeln!(out, "  </style>")?;
    writeln!(out, "</head>")?;
    writeln!(out, "<body>")?;
    writeln!(
        out,
        "  <h1>{}</h1>",
        html_escape(&root.display().to_string())
    )?;
    writeln!(out, "  <ul class=\"bush-tree\">")?;
    for child in &tree.children {
        render_html_node(out, child, 2)?;
    }
    writeln!(out, "  </ul>")?;
    let (dirs, files) = counts(tree);
    writeln!(
        out,
        "  <p class=\"bush-report\">{} director{}, {} file{}</p>",
        dirs,
        if dirs == 1 { "y" } else { "ies" },
        files,
        if files == 1 { "" } else { "s" }
    )?;
    writeln!(out, "</body>")?;
    writeln!(out, "</html>")?;
    Ok(())
}

fn render_html_node<W: Write>(out: &mut W, node: &Node, indent: usize) -> Result<()> {
    let pad = "  ".repeat(indent);
    let class = node_type(&node.meta);
    write!(
        out,
        "{pad}<li class=\"{}\"><span class=\"name\">{}</span>",
        class,
        html_escape(&node.name)
    )?;
    if let Some(target) = &node.meta.symlink_target {
        write!(
            out,
            " <span class=\"target\">-&gt; {}</span>",
            html_escape(&target.display().to_string())
        )?;
    }
    if node.children.is_empty() {
        writeln!(out, "</li>")?;
    } else {
        writeln!(out)?;
        writeln!(out, "{pad}  <ul>")?;
        for child in &node.children {
            render_html_node(out, child, indent + 2)?;
        }
        writeln!(out, "{pad}  </ul>")?;
        writeln!(out, "{pad}</li>")?;
    }
    Ok(())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// ===== XML =====

pub fn render_xml<W: Write>(root: &Path, tree: &Node, out: &mut W) -> Result<()> {
    writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    writeln!(
        out,
        "<tree root=\"{}\">",
        xml_escape(&root.display().to_string())
    )?;
    for child in &tree.children {
        render_xml_node(out, child, 1)?;
    }
    let (dirs, files) = counts(tree);
    writeln!(
        out,
        "  <report directories=\"{}\" files=\"{}\"/>",
        dirs, files
    )?;
    writeln!(out, "</tree>")?;
    Ok(())
}

fn render_xml_node<W: Write>(out: &mut W, node: &Node, indent: usize) -> Result<()> {
    let pad = "  ".repeat(indent);
    let tag = node_type(&node.meta);
    let name = xml_escape(&node.name);
    let mut attrs = format!(" name=\"{name}\"");
    if !node.meta.is_dir {
        attrs.push_str(&format!(" size=\"{}\"", node.meta.size));
    }
    if let Some(t) = node.meta.mtime {
        attrs.push_str(&format!(
            " mtime=\"{}\"",
            xml_escape(&format_meta::format_mtime(t))
        ));
    }
    if let Some(m) = node.meta.mode {
        attrs.push_str(&format!(
            " mode=\"{}\"",
            xml_escape(&format_meta::format_mode(m))
        ));
    }
    if let Some(target) = &node.meta.symlink_target {
        attrs.push_str(&format!(
            " target=\"{}\"",
            xml_escape(&target.display().to_string())
        ));
    }
    if node.children.is_empty() {
        writeln!(out, "{pad}<{tag}{attrs}/>")?;
    } else {
        writeln!(out, "{pad}<{tag}{attrs}>")?;
        for child in &node.children {
            render_xml_node(out, child, indent + 1)?;
        }
        writeln!(out, "{pad}</{tag}>")?;
    }
    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::prepare;
    use crate::tree::RenderOptions;
    use crate::walker::Entry;
    use std::path::PathBuf;

    fn entry(path: &str, is_dir: bool) -> Entry {
        Entry {
            path: PathBuf::from(path),
            is_dir,
            ..Default::default()
        }
    }

    fn render_json_str(root: &Path, entries: &[Entry]) -> String {
        let opts = RenderOptions::default();
        let tree = prepare(root, entries, &opts);
        let mut out = Vec::new();
        render_json(root, &tree, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn json_includes_root_and_report() {
        let s = render_json_str(Path::new("/r"), &[entry("/r/a", false)]);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["root"], "/r");
        assert_eq!(v["report"]["directories"], 0);
        assert_eq!(v["report"]["files"], 1);
    }

    #[test]
    fn json_children_have_name_and_type() {
        let s = render_json_str(
            Path::new("/r"),
            &[entry("/r/dir", true), entry("/r/dir/file.txt", false)],
        );
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        let children = v["children"].as_array().unwrap();
        assert_eq!(children[0]["name"], "dir");
        assert_eq!(children[0]["type"], "directory");
        let grand = children[0]["children"].as_array().unwrap();
        assert_eq!(grand[0]["name"], "file.txt");
        assert_eq!(grand[0]["type"], "file");
    }

    #[test]
    fn json_includes_size_for_files() {
        let entries = vec![Entry {
            path: PathBuf::from("/r/data.bin"),
            size: 12345,
            ..Default::default()
        }];
        let s = render_json_str(Path::new("/r"), &entries);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["children"][0]["size"], 12345);
    }

    #[test]
    fn json_symlink_node_has_target_and_type() {
        let entries = vec![Entry {
            path: PathBuf::from("/r/link"),
            is_symlink: true,
            symlink_target: Some(PathBuf::from("/real/target")),
            ..Default::default()
        }];
        let s = render_json_str(Path::new("/r"), &entries);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["children"][0]["type"], "symlink");
        assert_eq!(v["children"][0]["symlink_target"], "/real/target");
    }

    fn render_html_str(root: &Path, entries: &[Entry]) -> String {
        let opts = RenderOptions::default();
        let tree = prepare(root, entries, &opts);
        let mut out = Vec::new();
        render_html(root, &tree, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn html_has_doctype_and_body() {
        let s = render_html_str(Path::new("/r"), &[]);
        assert!(s.starts_with("<!DOCTYPE html>"));
        assert!(s.contains("<body>"));
        assert!(s.contains("</body>"));
    }

    #[test]
    fn html_renders_dir_and_file_classes() {
        let s = render_html_str(
            Path::new("/r"),
            &[entry("/r/dir", true), entry("/r/file.txt", false)],
        );
        assert!(s.contains("class=\"directory\""), "got: {s}");
        assert!(s.contains("class=\"file\""), "got: {s}");
    }

    #[test]
    fn html_escapes_special_chars() {
        let s = render_html_str(Path::new("/r"), &[entry("/r/<script>", false)]);
        assert!(s.contains("&lt;script&gt;"), "got: {s}");
        assert!(!s.contains("<script>"), "raw chars must not leak: {s}");
    }

    #[test]
    fn html_includes_report() {
        let s = render_html_str(Path::new("/r"), &[entry("/r/a", false)]);
        assert!(s.contains("0 directories, 1 file"), "got: {s}");
    }

    fn render_xml_str(root: &Path, entries: &[Entry]) -> String {
        let opts = RenderOptions::default();
        let tree = prepare(root, entries, &opts);
        let mut out = Vec::new();
        render_xml(root, &tree, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn xml_starts_with_declaration_and_root() {
        let s = render_xml_str(Path::new("/r"), &[]);
        assert!(s.starts_with("<?xml version=\"1.0\""));
        assert!(s.contains("<tree root=\"/r\""));
    }

    #[test]
    fn xml_files_and_dirs_have_appropriate_tags() {
        let s = render_xml_str(
            Path::new("/r"),
            &[entry("/r/dir", true), entry("/r/file.txt", false)],
        );
        assert!(s.contains("<directory name=\"dir\""), "got: {s}");
        assert!(s.contains("<file name=\"file.txt\""), "got: {s}");
    }

    #[test]
    fn xml_escapes_special_chars() {
        let s = render_xml_str(Path::new("/r"), &[entry("/r/<oops>", false)]);
        assert!(s.contains("&lt;oops&gt;"), "got: {s}");
    }

    #[test]
    fn xml_includes_report_element() {
        let s = render_xml_str(Path::new("/r"), &[entry("/r/a", false)]);
        assert!(
            s.contains("<report directories=\"0\" files=\"1\"/>"),
            "got: {s}"
        );
    }

    #[test]
    fn xml_file_has_size_attribute() {
        let entries = vec![Entry {
            path: PathBuf::from("/r/data"),
            size: 500,
            ..Default::default()
        }];
        let s = render_xml_str(Path::new("/r"), &entries);
        assert!(s.contains("size=\"500\""), "got: {s}");
    }
}
