use std::{fs::read_to_string, io::Error as IoError, path::Path};

use lnb_core::model::user_role::UserRole;
use regex::{Error as RegexError, Regex};
use serde::Deserialize;
use serde_json::Error as SerdeJsonError;
use thiserror::Error as ThisError;

#[derive(Debug, Clone)]
pub struct UserRoles {
    pub mastodon: UserRolesGroup,
    pub discord: UserRolesGroup,
}

#[derive(Debug, Clone)]
pub struct UserRolesGroup {
    default: UserRole,
    filters: Vec<RoleFilter>,
}

impl UserRolesGroup {
    pub fn get(&self, user: &str) -> &UserRole {
        self.filters
            .iter()
            .find_map(|f| f.matches(user))
            .unwrap_or(&self.default)
    }
}

#[derive(Debug, Clone)]
pub struct RoleFilter {
    pattern: Regex,
    role: UserRole,
}

impl RoleFilter {
    pub fn new(pattern: Regex, role: UserRole) -> RoleFilter {
        RoleFilter { pattern, role }
    }

    pub fn matches(&self, user: &str) -> Option<&UserRole> {
        self.pattern.is_match(user).then_some(&self.role)
    }
}

#[derive(Debug, Deserialize)]
struct UserRolesDefinition {
    pub mastodon: UserRolesGroupDefinition,
    pub discord: UserRolesGroupDefinition,
}

#[derive(Debug, Deserialize)]
struct UserRolesGroupDefinition {
    default: UserRole,
    filters: Vec<RolesFilterDefinition>,
}

#[derive(Debug, Deserialize)]
struct RolesFilterDefinition {
    pub user: Option<String>,
    pub pattern: Option<String>,
    role: UserRole,
}

impl TryFrom<UserRolesGroupDefinition> for UserRolesGroup {
    type Error = UserRolesError;

    fn try_from(value: UserRolesGroupDefinition) -> Result<UserRolesGroup, UserRolesError> {
        let filters: Result<Vec<_>, _> = value.filters.into_iter().map(|f| f.try_into()).collect();
        Ok(UserRolesGroup {
            default: value.default,
            filters: filters?,
        })
    }
}

impl TryFrom<RolesFilterDefinition> for RoleFilter {
    type Error = UserRolesError;

    fn try_from(value: RolesFilterDefinition) -> Result<RoleFilter, UserRolesError> {
        let filter_regex = match (value.user, value.pattern) {
            (None, Some(pattern)) => Regex::new(&pattern).map_err(UserRolesError::Regex)?,
            (Some(user), None) => {
                let pattern = format!("^{}$", regex::escape(&user));
                Regex::new(&pattern).map_err(UserRolesError::Regex)?
            }
            (Some(_), Some(_)) => {
                return Err(UserRolesError::Other("both user and pattern specified".to_string()));
            }
            (None, None) => return Err(UserRolesError::Other("user or pattern must exist".to_string())),
        };
        Ok(RoleFilter::new(filter_regex, value.role))
    }
}

pub fn load_user_roles(path: impl AsRef<Path>) -> Result<UserRoles, UserRolesError> {
    let config_str = read_to_string(path).map_err(UserRolesError::Io)?;
    let config: UserRolesDefinition = serde_json::from_str(&config_str).map_err(UserRolesError::Serialization)?;

    Ok(UserRoles {
        mastodon: config.mastodon.try_into()?,
        discord: config.discord.try_into()?,
    })
}

#[derive(Debug, ThisError)]
pub enum UserRolesError {
    #[error("io error: {0}")]
    Io(IoError),

    #[error("serialization error: {0}")]
    Serialization(SerdeJsonError),

    #[error("regex error: {0}")]
    Regex(RegexError),

    #[error("other error: {0}")]
    Other(String),
}
