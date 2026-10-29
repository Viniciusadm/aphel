use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::dto::{MeetingBookingListParams, MeetingBookingResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{
    OperationFuture, OperationPage, PluginOperation, operation_input, operation_output,
};

#[derive(Deserialize)]
struct BookingsInput {
    #[serde(flatten)]
    params: MeetingBookingListParams,
    #[serde(flatten)]
    page: OperationPage,
}

#[derive(Deserialize)]
struct CancelInput {
    id: String,
    #[serde(default)]
    reason: Option<String>,
}

fn bookings<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: BookingsInput = operation_input(input)?;
        let listing =
            service::list_bookings(ctx, input.params.into_query(), input.page.window()).await?;
        let bookings: Vec<MeetingBookingResponse> =
            listing.items.into_iter().map(Into::into).collect();
        Ok(json!({ "data": operation_output(bookings)?, "total": listing.total }))
    })
}

fn cancel<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: CancelInput = operation_input(input)?;
        let booking = service::cancel_booking(ctx, &input.id, input.reason, None).await?;
        operation_output(MeetingBookingResponse::from(booking))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "scheduling.bookings",
            "Meeting bookings of the hosts within the actor's scheduling.manage scope: { status?: confirmed|canceled, meeting_type_id?, upcoming?, page?, per_page? } -> { data, total }",
            bookings,
        )),
        Arc::new(PluginOperation::write(
            "scheduling.cancel",
            "Cancel a booking as the host (the guest receives the cancellation): { id, reason? }",
            cancel,
        )),
    ]
}
