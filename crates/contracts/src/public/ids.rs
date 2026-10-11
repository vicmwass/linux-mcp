//! Opaque identifiers, server revisions and UTC timestamps.
//!
//! Every ID is a distinct newtype over a UUID, so IDs cannot be mixed in Rust APIs:
//!
//! ```compile_fail
//! use linux_mcp_contracts::public::ids::{ServerId, UserId};
//! fn takes_server(_: ServerId) {}
//! let user = UserId::parse("6f9619ff-8b86-4011-b42d-00c04fc964ff").unwrap();
//! takes_server(user);
//! ```
//!
//! There is no conversion between ID types either:
//!
//! ```compile_fail
//! use linux_mcp_contracts::public::ids::{ServerId, UserId};
//! let user = UserId::parse("6f9619ff-8b86-4011-b42d-00c04fc964ff").unwrap();
//! let _server: ServerId = user.into();
//! ```
//!
//! Wire form (decisions.md D3): lowercase hyphenated UUID string; uppercase, braced,
//! URN, simple and nil forms are rejected.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use super::errors::{InvalidValue, ViolationRule};

/// Implements `schemars::JsonSchema` from a literal JSON schema (feature `schema`).
macro_rules! impl_json_schema {
    ($ty:ty, $name:expr, { $($json:tt)* }) => {
        #[cfg(feature = "schema")]
        impl schemars::JsonSchema for $ty {
            fn schema_name() -> std::borrow::Cow<'static, str> {
                $name.into()
            }

            fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
                schemars::json_schema!({ $($json)* })
            }
        }
    };
}
pub(crate) use impl_json_schema;

/// Regex for the canonical UUID wire form.
pub const UUID_PATTERN: &str = "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$";

fn parse_canonical_uuid(value: &str) -> Result<Uuid, InvalidValue> {
    let invalid = InvalidValue::new(ViolationRule::InvalidFormat);
    if value.len() != 36 {
        return Err(invalid);
    }
    let uuid = Uuid::try_parse(value).map_err(|_| invalid)?;
    if uuid.is_nil() || uuid.hyphenated().to_string() != value {
        return Err(invalid);
    }
    Ok(uuid)
}

macro_rules! opaque_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Uuid);

        impl $name {
            /// Parses the canonical lowercase hyphenated form.
            pub fn parse(value: &str) -> Result<Self, InvalidValue> {
                parse_canonical_uuid(value).map(Self)
            }

            /// Explicit construction from a stored UUID (for repository adapters). Rejects nil.
            pub fn from_uuid(uuid: Uuid) -> Result<Self, InvalidValue> {
                if uuid.is_nil() {
                    Err(InvalidValue::new(ViolationRule::InvalidFormat))
                } else {
                    Ok(Self(uuid))
                }
            }

            /// Generates a random (v4) identifier. Server-side only.
            #[cfg(feature = "backend")]
            pub fn new_random() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0.hyphenated(), f)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({})"), self.0.hyphenated())
            }
        }

        impl FromStr for $name {
            type Err = InvalidValue;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(&self.0.hyphenated())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
                Self::parse(&value).map_err(serde::de::Error::custom)
            }
        }

        impl_json_schema!($name, stringify!($name), {
            "type": "string",
            "format": "uuid",
            "pattern": UUID_PATTERN
        });
    };
}

opaque_id!(
    /// Installation-local user identifier (never an email or provider subject).
    UserId
);
opaque_id!(
    /// Identifier of one owned server record.
    ServerId
);
opaque_id!(
    /// Reference to a stored credential. Never a path and never secret content.
    CredentialId
);
opaque_id!(
    /// Identifier of one requested operation (intent).
    OperationId
);
opaque_id!(
    /// Identifier of one approval request.
    ApprovalId
);
opaque_id!(
    /// Correlation identifier returned with every response and error.
    RequestId
);
opaque_id!(
    /// Identifier of one installation (hosted or self-hosted). Tokens never cross installations.
    InstallationId
);
opaque_id!(
    /// Identifier of an enrolled host-trust record.
    HostTrustId
);
opaque_id!(
    /// Identifier of a durable audit record.
    AuditRecordId
);

/// Monotonic per-server revision. Starts at 1; bounded to `2^53 - 1` for JavaScript safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct ServerRevision(u64);

impl ServerRevision {
    pub const MAX_VALUE: u64 = (1 << 53) - 1;
    pub const INITIAL: ServerRevision = ServerRevision(1);

    pub fn new(value: u64) -> Result<Self, InvalidValue> {
        if (1..=Self::MAX_VALUE).contains(&value) {
            Ok(Self(value))
        } else {
            Err(InvalidValue::new(ViolationRule::OutOfRange))
        }
    }

    /// Checked increment; fails rather than wrapping or exceeding the wire bound.
    pub fn next(self) -> Result<Self, InvalidValue> {
        Self::new(self.0 + 1)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl TryFrom<u64> for ServerRevision {
    type Error = InvalidValue;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ServerRevision> for u64 {
    fn from(value: ServerRevision) -> Self {
        value.0
    }
}

impl fmt::Display for ServerRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl_json_schema!(ServerRevision, "ServerRevision", {
    "type": "integer",
    "minimum": 1,
    "maximum": 9007199254740991u64
});

/// UTC instant with millisecond precision; wire form RFC 3339 with `Z`, e.g. `2026-10-11T07:38:59.810Z`.
///
/// Sub-millisecond precision is truncated on construction so serialization round-trips exactly.
/// Contracts have no wall clock; adapters supply time (see test-support's controlled clock).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    pub fn from_datetime(value: DateTime<Utc>) -> Self {
        let millis = value.timestamp_millis();
        Self(DateTime::from_timestamp_millis(millis).expect("millisecond timestamp in range"))
    }

    pub fn from_unix_millis(millis: i64) -> Result<Self, InvalidValue> {
        DateTime::from_timestamp_millis(millis)
            .map(Self)
            .ok_or(InvalidValue::new(ViolationRule::OutOfRange))
    }

    /// Parses RFC 3339 with a `Z` designator (UTC only).
    pub fn parse(value: &str) -> Result<Self, InvalidValue> {
        // chrono also accepts a space separator; the wire form requires `T`.
        if !value.ends_with('Z') || value.as_bytes().get(10) != Some(&b'T') {
            return Err(InvalidValue::new(ViolationRule::InvalidFormat));
        }
        DateTime::parse_from_rfc3339(value)
            .map(|dt| Self::from_datetime(dt.with_timezone(&Utc)))
            .map_err(|_| InvalidValue::new(ViolationRule::InvalidFormat))
    }

    pub fn as_datetime(&self) -> DateTime<Utc> {
        self.0
    }

    pub fn unix_millis(&self) -> i64 {
        self.0.timestamp_millis()
    }

    /// Returns `self + millis`, or `None` on overflow.
    pub fn checked_add_millis(&self, millis: u64) -> Option<Self> {
        let millis = i64::try_from(millis).ok()?;
        self.unix_millis()
            .checked_add(millis)
            .and_then(|m| Self::from_unix_millis(m).ok())
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_rfc3339_opts(SecondsFormat::Millis, true))
    }
}

impl Serialize for Timestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

impl_json_schema!(Timestamp, "Timestamp", {
    "type": "string",
    "format": "date-time",
    "pattern": "Z$"
});
