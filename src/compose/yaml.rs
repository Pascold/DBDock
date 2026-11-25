//! A small YAML emitter for the subset Compose files need.
//!
//! Maps keep insertion order and every string scalar is written as a
//! double-quoted (JSON-compatible) string, so no value can ever be misread as
//! a number, boolean, null or something else by a YAML parser.

/// A YAML value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Yaml {
    Str(String),
    Int(i64),
    Bool(bool),
    Seq(Vec<Yaml>),
    Map(Vec<(String, Yaml)>),
}

impl Yaml {
    /// Empty map, filled with [`Yaml::insert`].
    pub fn map() -> Self {
        Yaml::Map(Vec::new())
    }

    /// Builder-style insert; panics only if `self` is not a map, which is a
    /// programming error caught by the tests.
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl Into<Yaml>) -> Self {
        self.insert(key, value);
        self
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<Yaml>) {
        match self {
            Yaml::Map(entries) => entries.push((key.into(), value.into())),
            other => unreachable!("insert on non-map YAML value {other:?}"),
        }
    }

    /// Sequence of strings.
    pub fn strings<I, S>(items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Yaml::Seq(items.into_iter().map(|s| Yaml::Str(s.into())).collect())
    }

    /// Serializes the value as a YAML document (without `---`).
    pub fn to_yaml_string(&self) -> String {
        let mut out = String::new();
        match self {
            Yaml::Map(entries) if !entries.is_empty() => write_map(&mut out, entries, 0),
            Yaml::Seq(items) if !items.is_empty() => write_seq(&mut out, items, 0),
            scalar => {
                write_inline(&mut out, scalar);
                out.push('\n');
            }
        }
        out
    }
}

impl From<&str> for Yaml {
    fn from(s: &str) -> Self {
        Yaml::Str(s.to_owned())
    }
}

impl From<String> for Yaml {
    fn from(s: String) -> Self {
        Yaml::Str(s)
    }
}

impl From<i64> for Yaml {
    fn from(n: i64) -> Self {
        Yaml::Int(n)
    }
}

impl From<bool> for Yaml {
    fn from(b: bool) -> Self {
        Yaml::Bool(b)
    }
}

fn is_block(value: &Yaml) -> bool {
    match value {
        Yaml::Map(entries) => !entries.is_empty(),
        Yaml::Seq(items) => !items.is_empty(),
        _ => false,
    }
}

fn write_map(out: &mut String, entries: &[(String, Yaml)], indent: usize) {
    for (i, (key, value)) in entries.iter().enumerate() {
        // The first key of a map inside a sequence follows "- " directly.
        if i > 0 || !out.ends_with("- ") {
            push_indent(out, indent);
        }
        write_key(out, key);
        out.push(':');
        if is_block(value) {
            out.push('\n');
            write_block(out, value, indent + 2);
        } else {
            out.push(' ');
            write_inline(out, value);
            out.push('\n');
        }
    }
}

fn write_seq(out: &mut String, items: &[Yaml], indent: usize) {
    for item in items {
        push_indent(out, indent);
        out.push('-');
        match item {
            Yaml::Map(entries) if !entries.is_empty() => {
                out.push(' ');
                write_map(out, entries, indent + 2);
            }
            Yaml::Seq(inner) if !inner.is_empty() => {
                out.push('\n');
                write_seq(out, inner, indent + 2);
            }
            scalar => {
                out.push(' ');
                write_inline(out, scalar);
                out.push('\n');
            }
        }
    }
}

fn write_block(out: &mut String, value: &Yaml, indent: usize) {
    match value {
        Yaml::Map(entries) => write_map(out, entries, indent),
        Yaml::Seq(items) => write_seq(out, items, indent),
        scalar => write_inline(out, scalar),
    }
}

/// Scalars and empty collections.
fn write_inline(out: &mut String, value: &Yaml) {
    match value {
        Yaml::Str(s) => write_quoted(out, s),
        Yaml::Int(n) => out.push_str(&n.to_string()),
        Yaml::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Yaml::Seq(_) => out.push_str("[]"),
        Yaml::Map(_) => out.push_str("{}"),
    }
}

fn write_key(out: &mut String, key: &str) {
    let plain = key
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
    if plain {
        out.push_str(key);
    } else {
        write_quoted(out, key);
    }
}

fn write_quoted(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn push_indent(out: &mut String, indent: usize) {
    out.extend(std::iter::repeat_n(' ', indent));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_structures() {
        let doc = Yaml::map()
            .with("name", "dbdock")
            .with(
                "services",
                Yaml::map().with(
                    "db",
                    Yaml::map()
                        .with("ports", Yaml::strings(["127.0.0.1:5432:5432"]))
                        .with(
                            "volumes",
                            Yaml::Seq(vec![Yaml::map()
                                .with("type", "bind")
                                .with("read_only", true)]),
                        )
                        .with("retries", 3)
                        .with("empty", Yaml::map()),
                ),
            )
            .with("volumes", Yaml::map().with("db-data", Yaml::map()));
        assert_eq!(
            doc.to_yaml_string(),
            "\
name: \"dbdock\"
services:
  db:
    ports:
      - \"127.0.0.1:5432:5432\"
    volumes:
      - type: \"bind\"
        read_only: true
    retries: 3
    empty: {}
volumes:
  db-data: {}
"
        );
    }

    #[test]
    fn strings_are_always_quoted_and_escaped() {
        let doc = Yaml::map()
            .with("a", "yes")
            .with("b", "C:\\Users\\x")
            .with("c", "say \"hi\"\nbye")
            .with("d", "zażółć")
            .with("weird key", "\u{7}");
        assert_eq!(
            doc.to_yaml_string(),
            "a: \"yes\"\nb: \"C:\\\\Users\\\\x\"\nc: \"say \\\"hi\\\"\\nbye\"\nd: \"zażółć\"\n\"weird key\": \"\\u0007\"\n"
        );
    }

    #[test]
    fn nested_sequences() {
        let doc = Yaml::Seq(vec![Yaml::strings(["a", "b"]), Yaml::Seq(vec![])]);
        assert_eq!(doc.to_yaml_string(), "-\n  - \"a\"\n  - \"b\"\n- []\n");
    }
}
