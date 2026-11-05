use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::dto::{MeetingBookingListParams, MeetingBookingResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, OperationPage, operation_input, operation_output};

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

const OP_BOOKING_BOOKINGS: OperationKey<Value, Value> = operation!("booking.bookings");
const OP_BOOKING_CANCEL: OperationKey<Value, Value> = operation!("booking.cancel");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_BOOKING_BOOKINGS, OperationSpec::read("Meeting bookings of the hosts within the actor's booking.manage scope: { status?: confirmed|canceled, meeting_type_id?, upcoming?, page?, per_page? } -> { data, total }"), bookings);
    app.operation(
        OP_BOOKING_CANCEL,
        OperationSpec::write(
            "Cancel a booking as the host (the guest receives the cancellation): { id, reason? }",
        )
        .durable(),
        cancel,
    );
}
