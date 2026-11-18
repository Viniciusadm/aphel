pub mod migrations;
pub mod pool;
pub mod transaction;

pub mod receipt;

pub use transaction::Tx;

/// Connection pool of the tenant database. Code outside the data layer names it
/// `Db`; only stores reach for the sqlx types behind it.
pub type Db = sqlx::MySqlPool;
