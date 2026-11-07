//! A minimal XML tree, enough to read an NF-e.
//!
//! Namespace prefixes are dropped from element and attribute names, comments and processing
//! instructions are ignored, and text is kept per element (`<a>x<b/>y</a>` -> `"xy"`).

use quick_xml::Reader;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::Event;

use crate::error::DanfeError;

/// An element with its attributes, children and text.
#[derive(Debug, Clone, Default)]
pub struct Node {
    /// Local name (no prefix).
    pub name: String,
    /// Attributes (no `xmlns`).
    pub attrs: Vec<(String, String)>,
    /// Child elements in document order.
    pub children: Vec<Node>,
    /// Concatenated character data of the element itself.
    pub text: String,
}

fn local_name(raw: &[u8]) -> String {
    let name = String::from_utf8_lossy(raw);
    match name.rsplit_once(':') {
        Some((_, local)) => local.to_owned(),
        None => name.into_owned(),
    }
}

impl Node {
    /// First child called `name`.
    pub fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    /// All children called `name`.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// Descends a `/`-separated path of first children.
    pub fn find(&self, path: &str) -> Option<&Node> {
        path.split('/')
            .try_fold(self, |node, name| node.child(name))
    }

    /// Trimmed text of the child `name` (empty when absent).
    pub fn text_of(&self, name: &str) -> &str {
        self.child(name).map_or("", |c| c.text.trim())
    }

    /// Value of attribute `name`.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

fn attributes(start: &quick_xml::events::BytesStart<'_>) -> Vec<(String, String)> {
    start
        .attributes()
        .flatten()
        .filter_map(|a| {
            let key = String::from_utf8_lossy(a.key.as_ref()).into_owned();
            if key == "xmlns" || key.starts_with("xmlns:") {
                return None;
            }
            let value = a
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .map(|v| v.into_owned())
                .unwrap_or_default();
            Some((local_name(key.as_bytes()), value))
        })
        .collect()
}

/// Parses a document and returns its root element.
pub fn parse(xml: &str) -> Result<Node, DanfeError> {
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<Node> = Vec::new();
    let mut root: Option<Node> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => stack.push(Node {
                name: local_name(e.name().as_ref()),
                attrs: attributes(&e),
                ..Node::default()
            }),
            Ok(Event::Empty(e)) => {
                let node = Node {
                    name: local_name(e.name().as_ref()),
                    attrs: attributes(&e),
                    ..Node::default()
                };
                attach(&mut stack, &mut root, node);
            }
            Ok(Event::End(_)) => {
                if let Some(node) = stack.pop() {
                    attach(&mut stack, &mut root, node);
                }
            }
            Ok(Event::Text(t)) => {
                if let (Some(top), Ok(text)) = (stack.last_mut(), t.decode()) {
                    top.text.push_str(&text);
                }
            }
            Ok(Event::CData(c)) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&String::from_utf8_lossy(c.as_ref()));
                }
            }
            Ok(Event::GeneralRef(r)) => {
                let Some(top) = stack.last_mut() else {
                    continue;
                };
                if let Ok(Some(ch)) = r.resolve_char_ref() {
                    top.text.push(ch);
                } else if let Ok(name) = r.decode() {
                    if let Some(resolved) = resolve_predefined_entity(&name) {
                        top.text.push_str(resolved);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(DanfeError::Xml(e.to_string())),
        }
    }
    if !stack.is_empty() {
        return Err(DanfeError::Xml("unexpected end of document".to_owned()));
    }
    root.ok_or_else(|| DanfeError::Xml("empty document".to_owned()))
}

fn attach(stack: &mut [Node], root: &mut Option<Node>, node: Node) {
    match stack.last_mut() {
        Some(parent) => parent.children.push(node),
        None => *root = Some(node),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_structure_text_and_attributes() {
        let root = parse(
            r#"<?xml version="1.0"?><n:a xmlns:n="u" id="1"><n:b>x &amp; y</n:b><c/><b>2</b></n:a>"#,
        )
        .expect("xml");
        assert_eq!(root.name, "a");
        assert_eq!(root.attr("id"), Some("1"));
        assert_eq!(root.attr("xmlns:n"), None);
        assert_eq!(root.text_of("b"), "x & y");
        assert_eq!(root.children_named("b").count(), 2);
        assert!(root.find("c").is_some());
        assert_eq!(root.text_of("missing"), "");
    }

    #[test]
    fn resolves_character_references_and_cdata() {
        let root = parse("<a><b>caf&#233; &#xE9;</b><c><![CDATA[<raw>]]></c></a>").expect("xml");
        assert_eq!(root.text_of("b"), "café é");
        assert_eq!(root.text_of("c"), "<raw>");
    }

    #[test]
    fn finds_nested_paths() {
        let root = parse("<a><b><c>1</c></b></a>").expect("xml");
        assert_eq!(root.find("b/c").map(|n| n.text.as_str()), Some("1"));
        assert!(root.find("b/x").is_none());
    }

    #[test]
    fn rejects_malformed_documents() {
        assert!(parse("<a><b></a>").is_err());
        assert!(parse("").is_err());
        assert!(parse("<a>").is_err());
    }
}
