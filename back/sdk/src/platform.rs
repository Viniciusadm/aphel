pub mod api {
    pub use aphel_kernel::api::{
        dto, error, extractors, payload_limit, rate_limit, record_services, routes, session_cookie,
        sso, state,
    };
}

pub mod application {
    pub use aphel_kernel::application::{
        access, audit, auth, custom_objects, directory, fields, files, installer, jobs, mail,
        organization_admin, organization_profile, outbound_http, public_events, query,
        record_sharing, secrets, security, shared,
    };
}

pub mod tenancy {
    pub use aphel_kernel::tenancy::{TenantLocal, plugin_management_enabled, spawn};
}

pub mod database {
    pub use aphel_kernel::database::{receipt, transaction};
}

pub mod domain {
    pub use aphel_kernel::domain::{
        access, custom_objects, fields, organization_profile, period, users,
    };
}

pub mod infrastructure {
    pub use aphel_kernel::infrastructure::{
        audit, auth, custom_objects, deferred_authority, events, fields, installer, jobs,
        organization_profile, outbound_http, platform, query, record_sharing, records,
        repositories, secrets, storage, transactions,
    };
}

pub mod modules {
    pub use aphel_kernel::modules::RouteTakeover;
    pub use aphel_kernel::modules::{
        app, builder, connect_with_modules, module_disabled, module_settings, operations,
        plugin_bridge, runtime, runtime_catalog, toggle_guard, try_app, unified_catalog,
    };
}

pub mod records {
    pub use aphel_kernel::records::{
        changes_payload, constrain_query, filter_search_plan, load_contributed, lock_version,
        normalize_domain, parse_record_id, permission_denied, serialize_records, snapshot,
    };
}

pub mod state {
    pub use aphel_kernel::state::{runtime_pool, runtime_state};
}
