mod http;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_sdk::AppError;
use aphel_sdk::platform::modules::builder::APHEL_VERSION;
pub const KEY: &str = "calendar";
pub struct CalendarModule;
impl Module for CalendarModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Calendar", APHEL_VERSION)
            .description("Calendar entries contributed by enabled record owners")
    }
    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contract(aphel_sdk::calendar::DESCRIPTOR);
        app.permission(
            aphel_sdk::platform::domain::access::PermissionDefinition::administrative(
                aphel_sdk::platform::domain::access::Permission::from_static("calendar.read"),
            ),
        )
        .grant_to(["admin", "manager", "member"]);
        app.openapi_tag("calendar", "Calendar entries from enabled providers")
            .routes(|_| http::router());
        Ok(())
    }
}
