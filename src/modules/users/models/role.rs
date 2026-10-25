use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The roles seeded by the `create_roles` migration. Stored as `roles.name`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum RoleName {
    /// Customer placing food orders.
    User,
    /// Restaurant manager handling menus and orders.
    Manager,
}

impl RoleName {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoleName::User => "user",
            RoleName::Manager => "manager",
        }
    }
}

impl fmt::Display for RoleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RoleName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(RoleName::User),
            "manager" => Ok(RoleName::Manager),
            other => Err(format!("unknown role `{other}`")),
        }
    }
}
