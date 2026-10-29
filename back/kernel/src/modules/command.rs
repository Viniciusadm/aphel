use std::error::Error;

use async_trait::async_trait;
use clap::{ArgMatches, Command};
use sqlx::MySqlPool;

use crate::config::AppConfig;

pub type CommandResult = Result<(), Box<dyn Error + Send + Sync>>;

#[async_trait]
pub trait ModuleCommand: Send + Sync {
    fn command(&self) -> Command;
    async fn run(&self, config: AppConfig, matches: &ArgMatches) -> CommandResult;
}

pub async fn connect_with_modules(
    database_url: &str,
) -> Result<MySqlPool, Box<dyn Error + Send + Sync>> {
    let pool = crate::database::pool::connect(database_url).await?;
    crate::modules::app().enabled().refresh(&pool).await?;
    Ok(pool)
}
