//! Minimal translation system.
//!
//! Texts live in `assets/i18n/<lang>.toml` (embedded into the binary). Nested
//! tables become dotted keys, so `[card] start = "Start"` is `card.start`.
//! Placeholders use `{name}` and are filled by [`tf`]. A missing key falls
//! back to English and then to the key itself, so a mistake never crashes.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

use crate::config::Language;

const EN: &str = include_str!("../../assets/i18n/en.toml");
const PL: &str = include_str!("../../assets/i18n/pl.toml");

type Table = HashMap<String, String>;

static CURRENT: AtomicU8 = AtomicU8::new(0);

fn tables() -> &'static [Table; 2] {
    static TABLES: OnceLock<[Table; 2]> = OnceLock::new();
    TABLES.get_or_init(|| [parse(EN), parse(PL)])
}

/// Flattens a TOML document into dotted keys. Invalid files yield an empty
/// table (the tests make sure the shipped files are valid).
fn parse(text: &str) -> Table {
    let mut table = Table::new();
    if let Ok(root) = toml::from_str::<toml::Table>(text) {
        flatten("", &root, &mut table);
    }
    table
}

fn flatten(prefix: &str, source: &toml::Table, out: &mut Table) {
    for (key, value) in source {
        let full = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            toml::Value::Table(inner) => flatten(&full, inner, out),
            toml::Value::String(s) => {
                out.insert(full, s.clone());
            }
            _ => {}
        }
    }
}

fn index(language: Language) -> usize {
    match language {
        Language::En => 0,
        Language::Pl => 1,
    }
}

pub fn set_language(language: Language) {
    CURRENT.store(index(language) as u8, Ordering::Relaxed);
}

pub fn language() -> Language {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Language::Pl,
        _ => Language::En,
    }
}

/// Language of the operating system (English unless it is Polish).
pub fn system_language() -> Language {
    sys_locale::get_locale()
        .map(|locale| Language::from_locale(&locale))
        .unwrap_or(Language::En)
}

/// Translated text for `key`.
pub fn t(key: &str) -> &'static str {
    let tables = tables();
    tables[usize::from(CURRENT.load(Ordering::Relaxed))]
        .get(key)
        .or_else(|| tables[0].get(key))
        .map(String::as_str)
        .unwrap_or("???")
}

/// Translated text with `{name}` placeholders replaced.
pub fn tf(key: &str, args: &[(&str, &dyn Display)]) -> String {
    let mut text = t(key).to_owned();
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), &value.to_string());
    }
    text
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn placeholders(text: &str) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        let mut rest = text;
        while let Some(start) = rest.find('{') {
            let Some(len) = rest[start..].find('}') else {
                break;
            };
            found.insert(rest[start + 1..start + len].to_owned());
            rest = &rest[start + len..];
        }
        found
    }

    #[test]
    fn files_are_valid_toml() {
        assert!(toml::from_str::<toml::Table>(EN).is_ok());
        assert!(toml::from_str::<toml::Table>(PL).is_ok());
    }

    #[test]
    fn languages_define_the_same_keys_and_placeholders() {
        let [en, pl] = tables();
        assert!(!en.is_empty());
        let en_keys: BTreeSet<_> = en.keys().collect();
        let pl_keys: BTreeSet<_> = pl.keys().collect();
        assert_eq!(
            en_keys.symmetric_difference(&pl_keys).collect::<Vec<_>>(),
            Vec::<&&String>::new()
        );
        for (key, text) in en {
            assert_eq!(placeholders(text), placeholders(&pl[key]), "{key}");
        }
    }

    /// Every `t("...")` / `tf("...")` in the sources must exist.
    #[test]
    fn all_used_keys_exist() {
        let [en, _] = tables();
        let mut missing = Vec::new();
        let mut stack = vec![std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")];
        while let Some(path) = stack.pop() {
            if path.is_dir() {
                for entry in std::fs::read_dir(&path).unwrap() {
                    stack.push(entry.unwrap().path());
                }
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            for marker in ["t(\"", "tf(\""] {
                for (pos, _) in source.match_indices(marker) {
                    let preceded_by_ident = source[..pos]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_alphanumeric() || c == '_');
                    if preceded_by_ident {
                        continue;
                    }
                    let start = pos + marker.len();
                    let Some(len) = source[start..].find('"') else {
                        continue;
                    };
                    let key = &source[start..start + len];
                    let looks_like_key = key.contains('.')
                        && key.starts_with(|c: char| c.is_ascii_lowercase())
                        && key.chars().all(|c| {
                            c.is_ascii_lowercase() || c.is_ascii_digit() || "._".contains(c)
                        });
                    if looks_like_key && !en.contains_key(key) {
                        missing.push(format!("{}: {key}", path.display()));
                    }
                }
            }
        }
        assert!(
            missing.is_empty(),
            "missing translations:\n{}",
            missing.join("\n")
        );
    }

    #[test]
    fn lookup_and_formatting() {
        set_language(Language::Pl);
        assert_eq!(language(), Language::Pl);
        assert_eq!(t("card.start"), "Uruchom");
        set_language(Language::En);
        assert_eq!(t("card.start"), "Start");
        let unknown = "no.such.key";
        assert_eq!(t(unknown), "???");
        assert_eq!(tf("card.port", &[("port", &5432)]), "port 5432");
    }
}
