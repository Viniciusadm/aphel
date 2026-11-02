use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn guard(request: Request, next: Next) -> Response {
    for policy in crate::modules::app().contributions::<dyn crate::modules::HttpRequestPolicy>() {
        if let Err(error) = policy.check(request.method(), request.uri().path()) {
            return crate::api::error::app_error(error);
        }
    }
    next.run(request).await
}
