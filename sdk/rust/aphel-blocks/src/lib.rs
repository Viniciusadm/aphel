pub mod blocks;
mod error;
pub mod slug;
pub mod style;
#[cfg(feature = "web")]
mod web;

pub use error::BlockError;
