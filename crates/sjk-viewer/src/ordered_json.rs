//! JSON kept in the order it was written: the blade-skin editor's view of a skin's files
//! (`crate::blade_editor`), read and written back without sorting their keys (the
//! workspace's `serde_json` keeps maps sorted), so a file saved from the editor reads as
//! its author laid it out. Numbers written without a fraction stay whole.

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use std::fmt::Write as _;

/// One JSON value, an object's members in their order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Node {
    Null,
    Bool(bool),
    /// A number written without a fraction or exponent.
    Int(i64),
    Float(f64),
    Text(String),
    List(Vec<Node>),
    Object(Vec<(String, Node)>),
}

/// One step into a value: an object's member or a list's item.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Step {
    Key(String),
    Index(usize),
}

impl Node {
    /// Read `text`, or why it does not read.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        serde_json::from_str(text).map_err(|error| error.to_string())
    }

    /// The member `key` of an object.
    pub(crate) fn get(&self, key: &str) -> Option<&Node> {
        match self {
            Self::Object(members) => members
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, node)| node),
            _ => None,
        }
    }

    /// The value at `path` under this one.
    #[cfg(test)]
    pub(crate) fn at(&self, path: &[Step]) -> Option<&Node> {
        path.iter().try_fold(self, |node, step| match (node, step) {
            (Self::Object(_), Step::Key(key)) => node.get(key),
            (Self::List(items), Step::Index(index)) => items.get(*index),
            _ => None,
        })
    }

    /// The value at `path` under this one, to change.
    pub(crate) fn at_mut(&mut self, path: &[Step]) -> Option<&mut Node> {
        path.iter().try_fold(self, |node, step| match (node, step) {
            (Self::Object(members), Step::Key(key)) => members
                .iter_mut()
                .find(|(name, _)| name == key)
                .map(|(_, node)| node),
            (Self::List(items), Step::Index(index)) => items.get_mut(*index),
            _ => None,
        })
    }

    /// Set member `key` of an object, in its place or at the end.
    pub(crate) fn insert(&mut self, key: &str, value: Node) {
        if let Self::Object(members) = self {
            match members.iter_mut().find(|(name, _)| name == key) {
                Some((_, node)) => *node = value,
                None => members.push((key.to_owned(), value)),
            }
        }
    }

    /// Take member `key` out of an object.
    pub(crate) fn remove(&mut self, key: &str) -> Option<Node> {
        let Self::Object(members) = self else {
            return None;
        };
        let at = members.iter().position(|(name, _)| name == key)?;
        Some(members.remove(at).1)
    }

    /// An object's members (none for anything else).
    pub(crate) fn members(&self) -> &[(String, Node)] {
        match self {
            Self::Object(members) => members,
            _ => &[],
        }
    }

    /// A number's value.
    pub(crate) fn number(&self) -> Option<f64> {
        match self {
            Self::Int(value) => Some(*value as f64),
            Self::Float(value) => Some(*value),
            _ => None,
        }
    }

    /// Whether this is a scalar (not a list or an object).
    fn scalar(&self) -> bool {
        !matches!(self, Self::List(_) | Self::Object(_))
    }

    /// The value on one line, as JSON.
    pub(crate) fn compact(&self) -> String {
        let mut out = String::new();
        self.write_compact(&mut out);
        out
    }

    fn write_compact(&self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Self::Int(value) => {
                let _ = write!(out, "{value}");
            }
            Self::Float(value) => out.push_str(&float(*value)),
            Self::Text(text) => {
                out.push_str(&serde_json::Value::String(text.clone()).to_string());
            }
            Self::List(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push_str(", ");
                    }
                    item.write_compact(out);
                }
                out.push(']');
            }
            Self::Object(members) => {
                out.push('{');
                for (index, (key, node)) in members.iter().enumerate() {
                    if index > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&serde_json::Value::String(key.clone()).to_string());
                    out.push_str(": ");
                    node.write_compact(out);
                }
                out.push('}');
            }
        }
    }

    /// The value as a file holds it: two spaces an indent, a list or an object of scalars
    /// that fits on a line kept on one, as the shipped skins write their colours and
    /// small sections.
    pub(crate) fn pretty(&self) -> String {
        let mut out = String::new();
        self.write_pretty(&mut out, 0);
        out.push('\n');
        out
    }

    fn write_pretty(&self, out: &mut String, depth: usize) {
        /// The longest line a list or object of scalars is kept on.
        const LINE: usize = 88;
        let inline = match self {
            Self::List(items) => items.iter().all(Node::scalar),
            Self::Object(members) => members.iter().all(|(_, node)| node.scalar()),
            _ => true,
        };
        let compact = inline.then(|| self.compact());
        if let Some(compact) = compact.filter(|text| depth * 2 + text.len() <= LINE) {
            out.push_str(&compact);
            return;
        }
        let indent = |out: &mut String, depth: usize| {
            for _ in 0..depth {
                out.push_str("  ");
            }
        };
        match self {
            Self::List(items) => {
                out.push_str("[\n");
                for (index, item) in items.iter().enumerate() {
                    indent(out, depth + 1);
                    item.write_pretty(out, depth + 1);
                    out.push_str(if index + 1 < items.len() { ",\n" } else { "\n" });
                }
                indent(out, depth);
                out.push(']');
            }
            Self::Object(members) => {
                out.push_str("{\n");
                for (index, (key, node)) in members.iter().enumerate() {
                    indent(out, depth + 1);
                    out.push_str(&serde_json::Value::String(key.clone()).to_string());
                    out.push_str(": ");
                    node.write_pretty(out, depth + 1);
                    out.push_str(if index + 1 < members.len() {
                        ",\n"
                    } else {
                        "\n"
                    });
                }
                indent(out, depth);
                out.push('}');
            }
            _ => self.write_compact(out),
        }
    }
}

/// `value` as the shortest decimal that reads back the same, with a fraction (`1.0`), so
/// a value written as a fraction stays one.
fn float(value: f64) -> String {
    if !value.is_finite() {
        return "0.0".to_owned();
    }
    let text = format!("{value}");
    if text.contains(['.', 'e', 'E']) {
        text
    } else {
        format!("{text}.0")
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct NodeVisitor;
        impl<'de> Visitor<'de> for NodeVisitor {
            type Value = Node;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON value")
            }

            fn visit_unit<E>(self) -> Result<Node, E> {
                Ok(Node::Null)
            }

            fn visit_bool<E>(self, value: bool) -> Result<Node, E> {
                Ok(Node::Bool(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Node, E> {
                Ok(Node::Int(value))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Node, E> {
                i64::try_from(value)
                    .map(Node::Int)
                    .map_err(|_| E::custom("a number too large"))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Node, E> {
                Ok(Node::Float(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Node, E> {
                Ok(Node::Text(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Node, E> {
                Ok(Node::Text(value))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Node, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(Node::List(items))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Node, A::Error> {
                let mut members: Vec<(String, Node)> = Vec::new();
                while let Some((key, value)) = map.next_entry::<String, Node>()? {
                    // A key given twice keeps its last value, in its first place.
                    match members.iter_mut().find(|(name, _)| *name == key) {
                        Some((_, node)) => *node = value,
                        None => members.push((key, value)),
                    }
                }
                Ok(Node::Object(members))
            }
        }
        deserializer.deserialize_any(NodeVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file reads back in its order, whole numbers staying whole and fractions
    /// fractions, colours and small sections on one line.
    #[test]
    fn a_file_reads_and_writes_in_its_own_order() {
        let text = r#"{"version": 1, "zeta": {"low": 0.2, "high": 1.0},
            "alpha": [0.9, 1.0, 1.0], "sounds": {"on": "a/b.wav", "swings": ["x", "y"]}}"#;
        let node = Node::parse(text).unwrap();
        let keys: Vec<_> = node.members().iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(keys, ["version", "zeta", "alpha", "sounds"]);
        let pretty = node.pretty();
        assert_eq!(
            pretty,
            "{\n  \"version\": 1,\n  \"zeta\": {\"low\": 0.2, \"high\": 1.0},\n  \"alpha\": [0.9, 1.0, 1.0],\n  \"sounds\": {\n    \"on\": \"a/b.wav\",\n    \"swings\": [\"x\", \"y\"]\n  }\n}\n"
        );
        assert_eq!(Node::parse(&pretty).unwrap(), node);
        let path = [Step::Key("zeta".into()), Step::Key("high".into())];
        assert_eq!(node.at(&path).and_then(Node::number), Some(1.0));
        let mut node = node;
        *node.at_mut(&path).unwrap() = Node::Float(0.75);
        node.insert("beta", Node::Bool(true));
        assert!(node.remove("alpha").is_some());
        assert_eq!(
            node.compact(),
            r#"{"version": 1, "zeta": {"low": 0.2, "high": 0.75}, "sounds": {"on": "a/b.wav", "swings": ["x", "y"]}, "beta": true}"#
        );
    }
}
