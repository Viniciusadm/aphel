use std::io::Write;

use clap::Args;

use crate::cli::CliResult;
use aphel_kernel::api;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Args)]
pub(crate) struct OpenapiArgs {
    #[arg(
        long,
        help = "Print the console API of multi-tenant installations instead"
    )]
    pub(crate) console: bool,
}

pub(crate) fn run(args: OpenapiArgs) -> CliResult {
    let document = if args.console {
        aphel_kernel::tenancy::console_openapi_json()?
    } else {
        api::openapi_json()?
    };
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(document.as_bytes())?;
    stdout.flush()?;
    Ok(())
}
