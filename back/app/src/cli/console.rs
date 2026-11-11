use clap::{Args, Subcommand};

use crate::cli::CliResult;
use aphel_kernel::config::{AppConfig, Tenancy};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::system::read_password;
use aphel_kernel::tenancy::{self, ControlPlane, OperatorActor};

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum ConsoleCommand {
    #[command(
        about = "Create an active console operator; the password is read without echo",
        long_about = "Create an active operator of the console served on CONSOLE_HOST. Use it for the first operator; the others are invited from the console. The password is read from the terminal without echo, or from standard input when it is not a terminal."
    )]
    CreateOperator(CreateOperatorArgs),
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct CreateOperatorArgs {
    #[arg(long, help = "Operator email, used to sign in")]
    pub(crate) email: String,
    #[arg(
        long,
        help = "Name shown in the console (default: the part of the email before @)"
    )]
    pub(crate) name: Option<String>,
}

pub(crate) async fn run(config: AppConfig, command: ConsoleCommand) -> CliResult {
    let Tenancy::Multi(multi) = config.tenancy()? else {
        return Err(AppError::Configuration(
            "console commands need CONTROL_DATABASE_URL".to_owned(),
        )
        .into());
    };
    let control = ControlPlane::connect(&multi).await?;
    let result = match command {
        ConsoleCommand::CreateOperator(args) => create_operator(&control, args).await,
    };
    control.close().await;
    result
}

async fn create_operator(control: &ControlPlane, args: CreateOperatorArgs) -> CliResult {
    control.migrate().await?;
    let name = args.name.clone().unwrap_or_else(|| {
        args.email
            .split('@')
            .next()
            .unwrap_or(args.email.as_str())
            .to_owned()
    });
    let password = read_password("Operator password: ")?;
    let confirmation = read_password("Repeat the password: ")?;
    if password != confirmation {
        return Err(AppError::Validation("the passwords do not match".to_owned()).into());
    }
    let operator = tenancy::create_operator(control.pool(), &args.email, &name, &password).await?;
    tenancy::record_operator_action(
        control.pool(),
        &OperatorActor::command_line(),
        "operator.created",
        None,
        serde_json::json!({ "operator_id": operator.id, "email": operator.email }),
    )
    .await;
    println!("Operator {} created.", operator.email);
    Ok(())
}
