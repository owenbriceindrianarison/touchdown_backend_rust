use std::fmt;

use shared::AppError;

/// Normalized email address (trimmed and lowercased at construction time).
/// Ensures no business logic ever operates on a raw, unnormalized string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    pub fn parse(raw: &str) -> Result<Self, AppError> {
        let normalized = raw.trim().to_lowercase();
        if normalized.is_empty() || !normalized.contains('@') {
            return Err(AppError::BadRequest("invalid email address".into()));
        }
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Email> for String {
    fn from(e: Email) -> String {
        e.0
    }
}
