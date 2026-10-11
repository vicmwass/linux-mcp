//! Deterministic canonicalization of operation arguments (`lmcp-canon-v1`, decisions.md D9).
//!
//! Rules:
//! - Objects: members sorted by the UTF-8 bytes of their keys; members whose value is
//!   `null` are removed; duplicate keys cannot occur (input is a parsed JSON value).
//! - No insignificant whitespace. Strings are emitted with `serde_json` escaping and are
//!   **not** Unicode-normalized, so visually similar strings remain distinct.
//! - Numbers: integers in `i64`/`u64` range only. Floats are rejected because their textual
//!   form is ambiguous across implementations.
//! - Limits: nesting depth ≤ [`MAX_DEPTH`], canonical size ≤ [`MAX_CANONICAL_BYTES`].
//! - Arguments must not contain secrets: keys matching [`SENSITIVE_KEY_FRAGMENTS`]
//!   (case-insensitive) are rejected at any depth.
//!
//! Digest: `sha256:` + lowercase hex of SHA-256 over `"lmcp-canon-v1\n" || canonical_json`.
//! Changing the algorithm requires a new version string, which changes every digest and so
//! safely invalidates approvals bound under the old version.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::public::errors::{InvalidValue, ViolationRule};
use crate::public::ids::impl_json_schema;

pub const CANONICALIZATION_VERSION: &str = "lmcp-canon-v1";
pub const MAX_DEPTH: usize = 16;
pub const MAX_CANONICAL_BYTES: usize = 64 * 1024;

/// Key fragments that indicate secret material. Matching is case-insensitive on the key with
/// `-` treated as `_`.
pub const SENSITIVE_KEY_FRAGMENTS: &[&str] = &[
    "password",
    "passphrase",
    "passwd",
    "secret",
    "private_key",
    "privatekey",
    "token",
    "api_key",
    "apikey",
    "credential",
];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CanonicalError {
    #[error("floating-point numbers are not allowed at {0}")]
    FloatNotAllowed(String),
    #[error("nesting deeper than {MAX_DEPTH} at {0}")]
    TooDeep(String),
    #[error("canonical form exceeds {MAX_CANONICAL_BYTES} bytes")]
    TooLarge,
    #[error("sensitive field is not allowed in operation arguments: {0}")]
    SensitiveField(String),
    #[error("arguments must be a JSON object")]
    NotAnObject,
}

impl CanonicalError {
    pub fn rule(&self) -> ViolationRule {
        match self {
            CanonicalError::FloatNotAllowed(_) | CanonicalError::NotAnObject => {
                ViolationRule::InvalidFormat
            }
            CanonicalError::TooDeep(_) | CanonicalError::TooLarge => ViolationRule::TooLong,
            CanonicalError::SensitiveField(_) => ViolationRule::NotAllowed,
        }
    }
}

/// Whether a key names secret material.
pub fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace('-', "_");
    SENSITIVE_KEY_FRAGMENTS
        .iter()
        .any(|fragment| normalized.contains(fragment))
}

/// Returns the canonical text of `value`.
pub fn canonicalize(value: &Value) -> Result<String, CanonicalError> {
    let mut out = String::new();
    write_value(value, &mut out, 0, "$")?;
    if out.len() > MAX_CANONICAL_BYTES {
        return Err(CanonicalError::TooLarge);
    }
    Ok(out)
}

/// Returns `value` with null object members removed (the value whose canonical text is
/// [`canonicalize`]'s output), after validating it.
pub fn normalize(value: &Value) -> Result<Value, CanonicalError> {
    canonicalize(value)?;
    Ok(strip_nulls(value))
}

fn strip_nulls(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), strip_nulls(v)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(strip_nulls).collect()),
        other => other.clone(),
    }
}

fn write_value(
    value: &Value,
    out: &mut String,
    depth: usize,
    path: &str,
) -> Result<(), CanonicalError> {
    if depth > MAX_DEPTH {
        return Err(CanonicalError::TooDeep(path.to_owned()));
    }
    if out.len() > MAX_CANONICAL_BYTES {
        return Err(CanonicalError::TooLarge);
    }
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                out.push_str(&i.to_string());
            } else if let Some(u) = n.as_u64() {
                out.push_str(&u.to_string());
            } else {
                return Err(CanonicalError::FloatNotAllowed(path.to_owned()));
            }
        }
        Value::String(s) => out.push_str(&serde_json::to_string(s).expect("string serializes")),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_value(item, out, depth + 1, &format!("{path}[{index}]"))?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut members: Vec<(&String, &Value)> =
                map.iter().filter(|(_, v)| !v.is_null()).collect();
            members.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
            out.push('{');
            for (index, (key, item)) in members.into_iter().enumerate() {
                let child = format!("{path}.{key}");
                if is_sensitive_key(key) {
                    return Err(CanonicalError::SensitiveField(child));
                }
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).expect("key serializes"));
                out.push(':');
                write_value(item, out, depth + 1, &child)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

/// `sha256:<64 lowercase hex>` digest of a canonical binding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IntentDigest(String);

impl IntentDigest {
    /// Digest of already-canonical text under the current version.
    pub fn of_canonical(canonical: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(CANONICALIZATION_VERSION.as_bytes());
        hasher.update(b"\n");
        hasher.update(canonical.as_bytes());
        let bytes = hasher.finalize();
        let mut text = String::with_capacity(7 + 64);
        text.push_str("sha256:");
        for byte in bytes {
            text.push_str(&format!("{byte:02x}"));
        }
        Self(text)
    }

    pub fn parse(value: &str) -> Result<Self, InvalidValue> {
        let hex = value
            .strip_prefix("sha256:")
            .ok_or(InvalidValue::new(ViolationRule::InvalidFormat))?;
        if hex.len() != 64
            || !hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(InvalidValue::new(ViolationRule::InvalidFormat));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for IntentDigest {
    type Error = InvalidValue;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<IntentDigest> for String {
    fn from(value: IntentDigest) -> Self {
        value.0
    }
}

impl fmt::Display for IntentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl_json_schema!(IntentDigest, "IntentDigest", {
    "type": "string",
    "pattern": "^sha256:[0-9a-f]{64}$"
});
