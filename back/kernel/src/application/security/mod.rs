pub mod account;
pub mod devices;
pub mod events;
pub mod lockout;
pub mod login;
pub mod notices;

pub use account::*;
pub use devices::*;
pub use events::*;
pub use lockout::*;
pub use login::*;
pub use notices::*;

use chrono::{DateTime, Utc};

use crate::domain::Secret;
use crate::errors::AppError;

pub trait PasswordHasher: Send + Sync {
    fn hash_password(&self, password: &str) -> Result<String, AppError>;
    fn verify_password(&self, password: &str, password_hash: &str) -> Result<bool, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedToken {
    pub token: Secret,
    pub hash: String,
}

pub trait SecureTokens: Send + Sync {
    fn issue(&self) -> Result<IssuedToken, AppError>;
    fn digest(&self, token: &str) -> String;
}

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub const MIN_PASSWORD_LENGTH: usize = 8;
pub const MAX_PASSWORD_LENGTH: usize = 128;

pub fn password_violation(password: &str) -> Option<String> {
    let length = password.chars().count();
    (!(MIN_PASSWORD_LENGTH..=MAX_PASSWORD_LENGTH).contains(&length)).then(|| {
        format!("must be between {MIN_PASSWORD_LENGTH} and {MAX_PASSWORD_LENGTH} characters")
    })
}

pub mod revocation;
pub use revocation::*;

pub mod signature;
