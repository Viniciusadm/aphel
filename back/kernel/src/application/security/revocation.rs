use crate::database::Db;
use crate::database::transaction::Tx;
use crate::domain::access::AccessScope;
use crate::domain::{SessionId, UserId};
use crate::errors::AppError;
use crate::modules::{Hook, HookCtx};

#[derive(Debug, Clone, Copy)]
pub struct SessionRevoked;
#[derive(Debug, Clone)]
pub struct SessionRevocation {
    pub session_id: Option<SessionId>,
    pub user_id: Option<UserId>,
    pub keep_session: Option<SessionId>,
}
impl Hook for SessionRevoked {
    type Args = SessionRevocation;
    fn key() -> &'static str {
        "session.revoked"
    }
}
pub async fn session_revoked(
    tx: &mut Tx,
    pool: &Db,
    session_id: Option<&SessionId>,
    user_id: Option<&UserId>,
    keep_session: Option<&SessionId>,
) -> Result<(), AppError> {
    let app = crate::modules::app();
    let scope = AccessScope::system();
    let mut ctx = HookCtx::new(tx, pool, &scope, app.services(), "kernel");
    app.hooks()
        .run(
            SessionRevoked,
            &mut ctx,
            &SessionRevocation {
                session_id: session_id.cloned(),
                user_id: user_id.cloned(),
                keep_session: keep_session.cloned(),
            },
        )
        .await
}
