use std::io::Write;

use crate::cli::CliResult;
use aphel_kernel::api;

pub(crate) fn run() -> CliResult {
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(api::openapi_json()?.as_bytes())?;
    stdout.flush()?;
    Ok(())
}
