use std::process::ExitCode;

#[cfg(feature = "plugins")]
#[path = "../../../blueprints/installer.rs"]
mod blueprints;
mod cli;
mod modules;

use cli::AlreadyReported;

#[tokio::main]
async fn main() -> ExitCode {
    match modules::build() {
        Ok(app) => {
            aphel_kernel::modules::install_app(app);
        }
        Err(error) => {
            eprintln!("Error: the modules of this build could not be loaded: {error}");
            return ExitCode::FAILURE;
        }
    }
    match cli::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.is::<AlreadyReported>() => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("Error: {error:?}");
            ExitCode::FAILURE
        }
    }
}
