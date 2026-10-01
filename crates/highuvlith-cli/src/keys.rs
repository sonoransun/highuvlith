//! Strict TOML keys.
//!
//! Two traps are closed here:
//!
//! 1. **Unknown keys** (typos, or Python keyword names that differ from the
//!    TOML names). Every table is deserialized with `deny_unknown_fields`;
//!    [`parse_toml`] rewrites serde's "unknown field" error into a message that
//!    names the key and its table, suggests the closest valid key, and lists
//!    the keys that are read in that context (e.g. by the selected
//!    `[source] type`).
//! 2. **Keys that exist but are not read** by the selected `type` / `mode`
//!    (e.g. a DPP key in an LPP source, or a Gaussian-bake key with
//!    `peb = "car"`). [`KeyRules::check`] rejects them, naming the key, the
//!    context that ignores it, where it *is* read, and the keys that are read.

use serde::de::DeserializeOwned;

/// Keys that are set in a typed table: every field that serializes to a
/// value. Meant for tables whose fields are all `Option`s (unset fields are
/// skipped by the TOML serializer), so the result is exactly the keys the
/// user wrote — under their canonical names, whatever alias was used.
pub fn present_keys<T: serde::Serialize>(value: &T) -> Vec<String> {
    match toml::Value::try_from(value) {
        Ok(toml::Value::Table(table)) => table.keys().cloned().collect(),
        _ => Vec::new(),
    }
}

/// Levenshtein edit distance (for did-you-mean suggestions).
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// The candidate closest to `unknown`, if it is plausibly a typo of it: a
/// case-insensitive match, a unit-suffixed version (`wavelength` →
/// `wavelength_nm`), or an edit distance of at most max(2, len/3).
pub fn did_you_mean<'a>(
    unknown: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let lower = unknown.to_ascii_lowercase();
    let candidates: Vec<&str> = candidates.into_iter().filter(|c| *c != unknown).collect();
    if let Some(c) = candidates.iter().find(|c| c.eq_ignore_ascii_case(unknown)) {
        return Some(c);
    }
    if let Some(c) = candidates
        .iter()
        .filter(|c| c.starts_with(&format!("{lower}_")))
        .min_by_key(|c| c.len())
    {
        return Some(c);
    }
    let limit = (unknown.chars().count() / 3).max(2);
    candidates
        .iter()
        .map(|c| (edit_distance(&lower, c), *c))
        .filter(|(d, _)| *d <= limit)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

/// The keys read in one context — a `[source] type`, an `[optics] type`, a
/// `[deep] mode`, ... — and the keys of that context that a condition
/// switches off (with the reason).
#[derive(Debug, Clone)]
pub struct KeyRules {
    /// Table name for messages, e.g. `[source]`.
    pub table: String,
    /// Context for messages, e.g. `type = "lpp"`.
    pub context: String,
    /// Keys read in this context.
    pub allowed: Vec<&'static str>,
    /// Keys of this context that are not read because of another setting,
    /// each with the reason.
    pub excluded: Vec<(&'static str, String)>,
}

impl KeyRules {
    pub fn new(table: &str, context: impl Into<String>, allowed: &[&'static str]) -> Self {
        Self {
            table: table.to_string(),
            context: context.into(),
            allowed: allowed.to_vec(),
            excluded: Vec::new(),
        }
    }

    /// Switch `key` off in this context, recording why.
    pub fn exclude(&mut self, key: &'static str, reason: impl Into<String>) {
        self.allowed.retain(|k| *k != key);
        if !self.excluded.iter().any(|(k, _)| *k == key) {
            self.excluded.push((key, reason.into()));
        }
    }

    /// Switch every key in `keys` off with the same reason.
    pub fn exclude_all(&mut self, keys: &[&'static str], reason: &str) {
        for key in keys {
            self.exclude(key, reason);
        }
    }

    /// `"[source] type = \"lpp\""`.
    pub fn label(&self) -> String {
        if self.context.is_empty() {
            self.table.clone()
        } else {
            format!("{} {}", self.table, self.context)
        }
    }

    /// Reject every key of `present` that this context does not read.
    /// `owners(key)` names the other contexts that do read a key (for the
    /// "it is read by ..." hint).
    pub fn check(
        &self,
        present: &[String],
        owners: &dyn Fn(&str) -> Vec<String>,
    ) -> anyhow::Result<()> {
        let mut problems = Vec::new();
        for key in present {
            if self.allowed.iter().any(|k| k == key) {
                continue;
            }
            if let Some((_, reason)) = self.excluded.iter().find(|(k, _)| k == key) {
                problems.push(format!("`{key}` is not used here: {reason}"));
                continue;
            }
            let who = owners(key);
            let mut line = if who.is_empty() {
                format!("`{key}` is not read by {}", self.label())
            } else {
                format!("`{key}` is only read by {}", who.join("; "))
            };
            if let Some(s) = did_you_mean(key, self.allowed.iter().copied()) {
                line.push_str(&format!(" — did you mean `{s}`?"));
            }
            problems.push(line);
        }
        if problems.is_empty() {
            return Ok(());
        }
        anyhow::bail!(
            "{} does not use {} of the given keys:\n  - {}\nkeys read by {}: {}",
            self.label(),
            problems.len(),
            problems.join("\n  - "),
            self.label(),
            self.allowed.join(", ")
        )
    }
}

/// A context hint for an unknown key: the keys read there (used for the
/// did-you-mean suggestion and listed in the message).
pub struct UnknownKeyHint {
    /// e.g. `[source] type = "lpp"`.
    pub label: String,
    pub keys: Vec<&'static str>,
}

/// Given the table path of an unknown key, the parsed document and the
/// field names serde expected there, describe the context (if the caller
/// knows it).
pub type HintFn<'a> = dyn Fn(&[String], &toml::Table, &[String]) -> Option<UnknownKeyHint> + 'a;

/// Parse `text` into `T`, turning an unknown-key error into a helpful
/// message (see the module docs). Other errors keep toml's own message,
/// which already points at the offending line and column.
pub fn parse_toml<T: DeserializeOwned>(text: &str, hint: &HintFn) -> anyhow::Result<T> {
    toml::from_str::<T>(text).map_err(|err| friendly_toml_error(&err, text, hint))
}

/// `unknown field `x`, expected ...` → `x`.
fn unknown_field(message: &str) -> Option<String> {
    let rest = message.strip_prefix("unknown field `")?;
    Some(rest[..rest.find('`')?].to_string())
}

/// The back-quoted names after "expected" in a serde unknown-field message.
fn expected_fields(message: &str) -> Vec<String> {
    let Some(pos) = message.find("expected") else {
        return Vec::new();
    };
    message[pos..]
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// Dotted path of the table whose body contains byte offset `pos`: the last
/// `[header]` / `[[header]]` line that starts before it. When the position is
/// on a header line whose last segment is the unknown `key` itself (an
/// unknown table name), the key belongs to the header's parent.
fn table_path_at(text: &str, pos: usize, key: &str) -> Vec<String> {
    let mut path = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if offset > pos {
            break;
        }
        let t = line.trim();
        if t.starts_with('[') {
            let inner = t.trim_start_matches('[');
            if let Some(end) = inner.find(']') {
                path = inner[..end]
                    .split('.')
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .collect();
                if pos < offset + line.len() && path.last().is_some_and(|p| p == key) {
                    path.pop();
                }
            }
        }
        offset += line.len();
    }
    path
}

fn friendly_toml_error(err: &toml::de::Error, text: &str, hint: &HintFn) -> anyhow::Error {
    let message = err.message();
    let full = err.to_string();
    let Some(key) = unknown_field(message) else {
        return anyhow::anyhow!("{}", full.trim_end());
    };
    let expected = expected_fields(message);
    let path = err
        .span()
        .map(|s| table_path_at(text, s.start, &key))
        .unwrap_or_default();
    let table = if path.is_empty() {
        "the top level".to_string()
    } else {
        format!("[{}]", path.join("."))
    };
    let context = toml::from_str::<toml::Table>(text)
        .ok()
        .and_then(|raw| hint(&path, &raw, &expected));
    let mut out = format!("unknown key `{key}` in {table}");
    let suggestion = context
        .as_ref()
        .and_then(|h| did_you_mean(&key, h.keys.iter().copied()))
        .or_else(|| did_you_mean(&key, expected.iter().map(String::as_str)));
    if let Some(s) = suggestion {
        out.push_str(&format!(" — did you mean `{s}`?"));
    }
    match &context {
        Some(h) => out.push_str(&format!(
            "\nkeys read by {}: {}",
            h.label,
            h.keys.join(", ")
        )),
        None if !expected.is_empty() => {
            out.push_str(&format!("\nvalid keys: {}", expected.join(", ")))
        }
        None => {}
    }
    anyhow::anyhow!("{}", full.replacen(message, &out, 1).trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_did_you_mean() {
        let keys = [
            "wavelength_nm",
            "collector_solid_angle_sr",
            "source_diameter_um",
            "sigma",
        ];
        assert_eq!(did_you_mean("wavelength", keys), Some("wavelength_nm"));
        assert_eq!(did_you_mean("Sigma", keys), Some("sigma"));
        assert_eq!(did_you_mean("sigam", keys), Some("sigma"));
        assert_eq!(
            did_you_mean("collector_solid_angle", keys),
            Some("collector_solid_angle_sr")
        );
        assert_eq!(
            did_you_mean("source_diameter_nm", keys),
            Some("source_diameter_um")
        );
        assert_eq!(did_you_mean("plasma_gun", keys), None);
    }

    #[test]
    fn test_table_path_and_expected_fields() {
        let text = "[source]\na = 1\n[[deep.filters]]\nb = 2\n";
        assert_eq!(table_path_at(text, 10, "a"), vec!["source"]);
        assert_eq!(
            table_path_at(text, text.len() - 2, "b"),
            vec!["deep", "filters"]
        );
        // On a header: an unknown table name belongs to the parent, an
        // unknown key of an array-of-tables element to the element.
        assert!(table_path_at(text, 1, "source").is_empty());
        assert_eq!(table_path_at(text, 18, "b"), vec!["deep", "filters"]);
        assert_eq!(table_path_at(text, 18, "filters"), vec!["deep"]);
        let msg = "unknown field `q`, expected one of `x`, `y`, `w`";
        assert_eq!(unknown_field(msg).as_deref(), Some("q"));
        assert_eq!(expected_fields(msg), vec!["x", "y", "w"]);
        assert_eq!(
            expected_fields("unknown field `z`, expected `a` or `b`"),
            vec!["a", "b"]
        );
    }

    #[test]
    fn test_key_rules_messages() {
        let mut rules = KeyRules::new("[source]", "type = \"lpp\"", &["type", "fuel", "sigma"]);
        rules.exclude("sigma", "the fill comes from [illumination]");
        let owners = |k: &str| {
            if k == "anode" {
                vec!["[source] type = \"xray_tube\"".to_string()]
            } else {
                Vec::new()
            }
        };
        rules
            .check(&["type".into(), "fuel".into()], &owners)
            .unwrap();
        let err = rules
            .check(&["anode".into(), "sigma".into(), "fuel".into()], &owners)
            .unwrap_err()
            .to_string();
        assert!(err.contains("does not use 2 of the given keys"), "{err}");
        assert!(
            err.contains("`anode` is only read by [source] type = \"xray_tube\""),
            "{err}"
        );
        assert!(
            err.contains("`sigma` is not used here: the fill comes from [illumination]"),
            "{err}"
        );
        assert!(
            err.contains("keys read by [source] type = \"lpp\": type, fuel"),
            "{err}"
        );
    }
}
