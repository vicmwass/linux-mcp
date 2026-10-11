//! Cursor pagination.
//!
//! Cursors are opaque, server-generated strings (base64url alphabet, 1–512 characters).
//! Implementations must bind a cursor to the principal, installation and query that
//! produced it (for example with a MAC) and treat a cursor from another principal as a
//! `validation` error, never as a way to page through another owner's data.

use serde::{Deserialize, Serialize};

use super::errors::{InvalidValue, ViolationRule};
use super::ids::impl_json_schema;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PageCursor(String);

impl PageCursor {
    pub const MAX_LEN: usize = 512;

    pub fn new(value: impl Into<String>) -> Result<Self, InvalidValue> {
        let value = value.into();
        if value.is_empty() {
            return Err(InvalidValue::new(ViolationRule::TooShort));
        }
        if value.len() > Self::MAX_LEN {
            return Err(InvalidValue::new(ViolationRule::TooLong));
        }
        if !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(InvalidValue::new(ViolationRule::InvalidFormat));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PageCursor {
    type Error = InvalidValue;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<PageCursor> for String {
    fn from(value: PageCursor) -> Self {
        value.0
    }
}

impl_json_schema!(PageCursor, "PageCursor", {
    "type": "string",
    "minLength": 1,
    "maxLength": 512,
    "pattern": "^[A-Za-z0-9_-]+$"
});

/// Page size, 1–200, default 50.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct PageLimit(u16);

impl PageLimit {
    pub const MAX: u16 = 200;
    pub const DEFAULT: PageLimit = PageLimit(50);

    pub fn new(value: u16) -> Result<Self, InvalidValue> {
        if (1..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(InvalidValue::new(ViolationRule::OutOfRange))
        }
    }

    pub fn get(self) -> u16 {
        self.0
    }
}

impl Default for PageLimit {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl TryFrom<u16> for PageLimit {
    type Error = InvalidValue;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<PageLimit> for u16 {
    fn from(value: PageLimit) -> Self {
        value.0
    }
}

impl_json_schema!(PageLimit, "PageLimit", {
    "type": "integer",
    "minimum": 1,
    "maximum": 200,
    "default": 50
});

/// Query parameters for any list operation (`?cursor=…&limit=…`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PageRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<PageCursor>,
    #[serde(default)]
    pub limit: PageLimit,
}

/// One page of results. `next_cursor` is `null` on the last page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<PageCursor>,
}

impl<T> Page<T> {
    pub fn last(items: Vec<T>) -> Self {
        Self {
            items,
            next_cursor: None,
        }
    }

    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            next_cursor: self.next_cursor,
        }
    }
}
