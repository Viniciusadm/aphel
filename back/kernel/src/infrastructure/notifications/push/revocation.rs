use sqlx::{Executor, MySql};

use crate::domain::{SessionId, UserId};
use crate::errors::AppError;

pub(crate) async fn remove_session_push_subscriptions<'c, E>(
    executor: E,
    session_id: &SessionId,
) -> Result<(), AppError>
where
    E: Executor<'c, Database = MySql>,
{
    sqlx::query("DELETE FROM push_subscriptions WHERE session_id = ?")
        .bind(session_id.as_str())
        .execute(executor)
        .await?;
    Ok(())
}

pub(crate) async fn remove_user_push_subscriptions<'c, E>(
    executor: E,
    user_id: &UserId,
    keep_session: Option<&SessionId>,
) -> Result<(), AppError>
where
    E: Executor<'c, Database = MySql>,
{
    match keep_session {
        Some(keep) => {
            sqlx::query("DELETE FROM push_subscriptions WHERE user_id = ? AND session_id <> ?")
                .bind(user_id.as_str())
                .bind(keep.as_str())
                .execute(executor)
                .await?;
        }
        None => {
            sqlx::query("DELETE FROM push_subscriptions WHERE user_id = ?")
                .bind(user_id.as_str())
                .execute(executor)
                .await?;
        }
    }
    Ok(())
}
