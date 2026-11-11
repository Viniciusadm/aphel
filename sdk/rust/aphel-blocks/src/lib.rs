pub mod blocks;
mod error;
pub mod slug;
#[cfg(feature = "web")]
mod web;

pub use error::BlockError;
