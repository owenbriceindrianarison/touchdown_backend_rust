use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    #[default]
    EN,
    FR,
}

impl Locale {
    pub const ALL: [Locale; 2] = [Locale::EN, Locale::FR];

    pub fn as_str(&self) -> &'static str {
        match self {
            Locale::EN => "en",
            Locale::FR => "fr",
        }
    }

    /// Negotiation based on an `Accept-Language` header, with no external dependencies.
    /// Tolerant: `fr-FR,fr;q=0.9,en;q=0.8` → French. Unknown → default.
    pub fn from_accept_language(header: &str, default: Locale) -> Locale {
        header
            .split(',')
            .filter_map(|part| {
                let tag = part.split(';').next()?.trim();
                tag.split('-').next()?.parse::<Locale>().ok()
            })
            .next()
            .unwrap_or(default)
    }
}

impl FromStr for Locale {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "en" => Ok(Locale::EN),
            "fr" => Ok(Locale::FR),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
