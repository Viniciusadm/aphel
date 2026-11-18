use std::time::Duration;

use serde_json::json;

use crate::application::system::updates::SystemUpdateStatus;
use crate::errors::AppError;

pub async fn request(
    action: &str,
    actor: Option<&str>,
    operation_id: Option<&str>,
) -> Result<SystemUpdateStatus, AppError> {
    let Some(socket) = std::env::var_os("APHEL_UPDATER_SOCKET").filter(|value| !value.is_empty())
    else {
        return if action == "status" {
            Ok(SystemUpdateStatus::default())
        } else {
            Err(unavailable())
        };
    };
    #[cfg(not(unix))]
    {
        let _ = (socket, actor, operation_id);
        Err(unavailable())
    }
    #[cfg(unix)]
    {
        let client = reqwest::Client::builder()
            .unix_socket(std::path::PathBuf::from(socket))
            .no_proxy()
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|_| unavailable())?;
        let url = format!("http://localhost/{action}");
        let response = if action == "status" {
            client.get(url)
        } else {
            client.post(url).json(&json!({
                "actor": actor,
                "operation_id": operation_id,
            }))
        }
        .send()
        .await
        .map_err(|_| unavailable())?;
        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err(AppError::conflict_code(
                "system_update_conflict",
                "the update state changed; refresh before trying again",
            ));
        }
        if !response.status().is_success() {
            return Err(unavailable());
        }
        let mut status: SystemUpdateStatus = response.json().await.map_err(|_| unavailable())?;
        status.current_version = crate::application::system::APHEL_VERSION.to_owned();
        Ok(status)
    }
}

fn unavailable() -> AppError {
    AppError::unavailable_code(
        "system_updater_unavailable",
        "the external update service is not available",
    )
}
