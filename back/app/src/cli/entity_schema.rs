use std::path::PathBuf;

use aphel_kernel::application::fields::EntityDeclaration;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::infrastructure::fields::managed_schema;
use clap::Args;

use super::CliResult;

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct EntitySchemaArgs {
    #[arg(long)]
    pub(crate) key: String,
    pub(crate) definition: PathBuf,
}

pub(crate) fn run(args: EntitySchemaArgs) -> CliResult {
    let declaration: EntityDeclaration = serde_json::from_slice(&std::fs::read(args.definition)?)?;
    let permission = Permission::parse(format!("{}.read", args.key)).map_err(|error| {
        aphel_kernel::errors::AppError::Validation(format!("invalid entity permission: {error:?}"))
    })?;
    let definition = declaration.definition(&args.key, permission)?;
    for statement in managed_schema(&definition)? {
        println!("{statement};");
    }
    Ok(())
}
