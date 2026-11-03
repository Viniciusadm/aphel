mod http;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_sdk::AppError;
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
            aphel_kernel::domain::access::PermissionDefinition::administrative(
                aphel_kernel::domain::access::Permission::from_static("calendar.read"),
            ),
        )
        .grant_to(["admin", "manager", "member"]);
        app.openapi_tag("calendar", "Calendar entries from enabled providers")
            .routes(|_| http::router());
        Ok(())
    }
}
