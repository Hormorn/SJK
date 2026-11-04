//! What the blade-skin format holds, read from the format itself (`crate::blade_skin_file`):
//! the blade-skin editor's sections and the values a section starts from
//! (`crate::blade_editor`), so that the editor covers whatever the format can draw
//! without a list of its own. A skin is read with a probe in it, and the reading's own
//! words say what is wanted there: a member it does not know lists the members it does,
//! in the format's order ("unknown field `?`, expected one of `count`, `color`, ..."); a
//! value of the wrong type names the type ("expected struct Span", "expected an array of
//! length 3", "expected u32"); a value out of its range names the range ("arcs.width must
//! be 0.02 to 1, not 0.5"). A section is built field by field from those words, then its
//! values moved into their ranges until it reads.

use crate::ordered_json::{Node, Step};

/// A member no format has: what the probes ask for.
const PROBE: &str = "?";
/// Most rounds of moving values into their ranges.
const ROUNDS: usize = 400;

/// The sections a skin may hold, as the format names them and in its order, read with
/// `skin` (the files laid over each other, which must read as JSON the format takes).
pub(crate) fn sections(id: &str, skin: &Node) -> Vec<String> {
    let mut probed = skin.clone();
    probed.insert(PROBE, Node::Int(0));
    match read(id, &probed) {
        Err(why) if why.starts_with("unknown field") => expected(&why),
        _ => Vec::new(),
    }
}

/// Section `key` as the format wants it, made from what reading `skin` with it says: its
/// fields in the format's order, each of the type wanted, values within their ranges.
/// The section may still not read when a rule binds two of its values together; the
/// editor then says why.
pub(crate) fn section(id: &str, skin: &Node, key: &str) -> Node {
    let mut builder = Builder {
        id,
        skin: skin.clone(),
        key,
    };
    let section = builder.value(&[]);
    builder.within(section)
}

struct Builder<'a> {
    id: &'a str,
    skin: Node,
    key: &'a str,
}

impl Builder<'_> {
    /// Why the skin does not read with `section` in it, if it does not.
    fn try_section(&self, section: &Node) -> Option<String> {
        let mut skin = self.skin.clone();
        skin.insert(self.key, section.clone());
        read(self.id, &skin).err()
    }

    /// A value of the type the format wants at `path` in the section.
    fn value(&mut self, path: &[Step]) -> Node {
        // A number fits most fields; what the format says to that tells the others.
        let why = self.try_at(path, Node::Float(0.5));
        let Some(why) = why.filter(|why| why.starts_with("invalid type")) else {
            return Node::Float(0.5);
        };
        let wanted = why.split("expected ").nth(1).unwrap_or_default();
        if wanted.starts_with("struct") || wanted.starts_with("a map") {
            self.object(path)
        } else if let Some(length) = wanted
            .strip_prefix("an array of length ")
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|length| length.parse::<usize>().ok())
        {
            let item = self.item(path);
            Node::List(vec![item; length])
        } else if wanted.starts_with("a sequence") {
            Node::List(vec![self.item(path)])
        } else if wanted.starts_with('u') || wanted.starts_with('i') || wanted.contains("integer") {
            Node::Int(1)
        } else if wanted.starts_with("a string") {
            Node::Text(String::new())
        } else if wanted.starts_with("a boolean") {
            Node::Bool(false)
        } else {
            Node::Float(0.5)
        }
    }

    /// A list's item at `path`, of the type the format wants there.
    fn item(&mut self, path: &[Step]) -> Node {
        let mut inner = path.to_vec();
        inner.push(Step::Index(0));
        self.value(&inner)
    }

    /// An object at `path` with every member the format allows there, each of its type.
    fn object(&mut self, path: &[Step]) -> Node {
        let probe = Node::Object(vec![(PROBE.to_owned(), Node::Int(0))]);
        let fields = self
            .try_at(path, probe)
            .filter(|why| why.starts_with("unknown field"))
            .map(|why| expected(&why))
            .unwrap_or_default();
        let mut members = Vec::new();
        for field in fields {
            let mut inner = path.to_vec();
            inner.push(Step::Key(field.clone()));
            members.push((field, self.value(&inner)));
        }
        Node::Object(members)
    }

    /// Why the skin does not read with `value` alone at `path` of the section (every
    /// object on the way holding only what leads there), if it does not.
    fn try_at(&self, path: &[Step], value: Node) -> Option<String> {
        let section = path.iter().rev().fold(value, |inner, step| match step {
            Step::Key(key) => Node::Object(vec![(key.clone(), inner)]),
            Step::Index(_) => Node::List(vec![inner]),
        });
        self.try_section(&section)
    }

    /// `section` with each value its range refuses moved into the range, until it reads
    /// or something other than a range refuses it.
    fn within(&self, mut section: Node) -> Node {
        for _ in 0..ROUNDS {
            let Some(why) = self.try_section(&section) else {
                break;
            };
            let Some((name, low, high)) = refused(&why) else {
                break;
            };
            let Some(path) = path_of(self.key, &name) else {
                break;
            };
            let Some(slot) = section.at_mut(&path) else {
                break;
            };
            *slot = match slot {
                Node::Int(_) => Node::Int(low.ceil() as i64),
                // Inside the range, not at an end, so that a rule between two values
                // has room.
                _ => Node::Float(((low + (high - low) * 0.5) * 1e4).round() / 1e4),
            };
        }
        section
    }
}

/// Why `skin` does not read as skin `id`, if it does not.
fn read(id: &str, skin: &Node) -> Result<(), String> {
    crate::blade_skin_file::parse(id, &skin.compact()).map(|_| ())
}

/// The names an "expected one of `a`, `b`" (or "expected `a` or `b`") lists.
fn expected(why: &str) -> Vec<String> {
    let listed = why.split("expected").nth(1).unwrap_or_default();
    listed
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// The field, low and high of "`field` must be `low` to `high`, not `value`".
fn refused(why: &str) -> Option<(String, f64, f64)> {
    let (name, rest) = why.split_once(" must be ")?;
    let (low, rest) = rest.split_once(" to ")?;
    let (high, _) = rest.split_once(", not ")?;
    Some((name.to_owned(), low.parse().ok()?, high.parse().ok()?))
}

/// The path in section `key` of a field the checks name (`arcs.span.low`,
/// `granulation.octaves[0].scale`, `motes.color red`); `None` for another section's.
fn path_of(key: &str, name: &str) -> Option<Vec<Step>> {
    let (name, channel) = match name.rsplit_once(' ') {
        Some((name, "red")) => (name, Some(0)),
        Some((name, "green")) => (name, Some(1)),
        Some((name, "blue")) => (name, Some(2)),
        _ => (name, None),
    };
    let mut parts = name.split('.');
    let first = parts.next()?;
    let (first, mut indices) = split_indices(first);
    if first != key {
        return None;
    }
    let mut path: Vec<Step> = indices.drain(..).map(Step::Index).collect();
    for part in parts {
        let (part, indices) = split_indices(part);
        path.push(Step::Key(part.to_owned()));
        path.extend(indices.into_iter().map(Step::Index));
    }
    path.extend(channel.map(Step::Index));
    Some(path)
}

/// `octaves[0]` as `octaves` and `[0]`.
fn split_indices(part: &str) -> (&str, Vec<usize>) {
    let mut pieces = part.split('[');
    let name = pieces.next().unwrap_or_default();
    let indices = pieces
        .filter_map(|piece| piece.trim_end_matches(']').parse().ok())
        .collect();
    (name, indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blade_skin_file::tests::SAMPLE;

    /// The sections are the format's, in its order; each is made from what the format
    /// says, and every one of them reads in the test sample, but for the few whose rules
    /// bind values together or name game paths, which the editor then shows.
    #[test]
    fn every_section_is_made_from_the_format() {
        let skin = Node::parse(SAMPLE).unwrap();
        let sections = sections("saber_sun", &skin);
        assert_eq!(sections.first().map(String::as_str), Some("version"));
        assert!(sections.iter().any(|key| key == "holes"));
        assert!(sections.len() >= 48, "{sections:?}");
        let mut unread = Vec::new();
        for key in sections.iter().filter(|key| skin.get(key).is_none()) {
            let made = section("saber_sun", &skin, key);
            assert!(
                matches!(made, Node::Object(_) | Node::List(_) | Node::Text(_)),
                "{key}: {made:?}"
            );
            let mut with = skin.clone();
            with.insert(key, made);
            if let Err(why) = read("saber_sun", &with) {
                unread.push(format!("{key}: {why}"));
            }
        }
        println!("{unread:#?}");
        // Those naming game paths: the images, smoke, sparks, bolts and the crackle.
        assert!(unread.len() <= 9, "{unread:#?}");
        let arcs = section("saber_sun", &skin, "arcs");
        assert!(matches!(arcs.get("color"), Some(Node::List(items)) if items.len() == 3));
        assert!(matches!(arcs.get("span"), Some(Node::Object(_))));
        assert!(matches!(arcs.get("count"), Some(Node::Int(_))));
    }

    #[test]
    fn paths_follow_the_checks_names() {
        assert_eq!(
            path_of("granulation", "granulation.octaves[1].scale"),
            Some(vec![
                Step::Key("octaves".into()),
                Step::Index(1),
                Step::Key("scale".into())
            ])
        );
        assert_eq!(
            path_of("motes", "motes.color red"),
            Some(vec![Step::Key("color".into()), Step::Index(0)])
        );
        assert_eq!(path_of("arcs", "motes.size"), None);
        assert_eq!(
            refused("arcs.width must be 0.02 to 1, not 0.5"),
            Some(("arcs.width".to_owned(), 0.02, 1.0))
        );
        assert_eq!(
            expected("unknown field `?`, expected `low` or `high` at line 1"),
            ["low", "high"]
        );
    }
}
