use sqlx::{MySql, Transaction};

pub type DatabaseTransaction<'a> = Transaction<'a, MySql>;

pub type MySqlTx = DatabaseTransaction<'static>;
