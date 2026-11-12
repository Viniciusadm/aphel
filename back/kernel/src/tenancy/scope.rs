use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::{Body, Bytes};
use axum::response::Response;
use futures::future::Either;
use http_body::{Body as HttpBody, Frame, SizeHint};
use tokio::task::JoinHandle;
use tokio::task::futures::TaskLocalFuture;

use crate::state::AppState;
use crate::tenancy::TenantRuntime;

tokio::task_local! {
    static TENANT: Arc<TenantRuntime>;
}

pub fn current() -> Option<Arc<TenantRuntime>> {
    TENANT.try_with(Arc::clone).ok()
}

pub fn plugin_management_enabled() -> bool {
    TENANT
        .try_with(|tenant| tenant.plugin_management_enabled())
        .unwrap_or(true)
}

pub fn current_state() -> Option<AppState> {
    TENANT
        .try_with(|tenant| tenant.state().cloned())
        .ok()
        .flatten()
}

pub async fn scope<F: Future>(tenant: Arc<TenantRuntime>, future: F) -> F::Output {
    TENANT.scope(tenant, future).await
}

pub fn bind<F>(future: F) -> Either<TaskLocalFuture<Arc<TenantRuntime>, F>, F>
where
    F: Future,
{
    match current() {
        Some(tenant) => Either::Left(TENANT.scope(tenant, future)),
        None => Either::Right(future),
    }
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    match current() {
        Some(tenant) => {
            let task = tokio::spawn(TENANT.scope(tenant.clone(), future));
            tenant.track(task.abort_handle());
            task
        }
        None => tokio::spawn(future),
    }
}

pub(super) fn scoped_response(tenant: Arc<TenantRuntime>, response: Response) -> Response {
    response.map(|inner| Body::new(ScopedBody { inner, tenant }))
}

struct ScopedBody {
    inner: Body,
    tenant: Arc<TenantRuntime>,
}

impl HttpBody for ScopedBody {
    type Data = Bytes;
    type Error = axum::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        let inner = Pin::new(&mut this.inner);
        TENANT.sync_scope(this.tenant.clone(), || inner.poll_frame(context))
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}
