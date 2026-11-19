export interface paths {
    "/api/console/v1/audit": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["console_audit"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/catalog": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["console_catalog"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/invitations/{token}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["console_invitation"];
        put?: never;
        post: operations["console_accept_invitation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/operators": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["console_list_operators"];
        put?: never;
        post: operations["console_invite_operator"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/operators/{id}/deactivate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["console_deactivate_operator"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/operators/{id}/invitation": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["console_renew_invitation"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/session": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["console_session"];
        put?: never;
        post: operations["console_sign_in"];
        delete: operations["console_sign_out"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/session/password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["console_change_password"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["console_list_tenants"];
        put?: never;
        /** @description With mode blank, only host is required: an internal identifier and console label are generated, the isolated database is migrated and the normal installer stays available on that host. No organization, administrator, optional modules or blueprint are installed. With mode configured, the supplied organization, administrator and configuration are installed during provisioning. */
        post: operations["console_create_tenant"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["console_tenant"];
        put?: never;
        post?: never;
        delete: operations["console_delete_tenant"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/features": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put: operations["console_update_tenant_features"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/health": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["console_tenant_health"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/hosts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["console_add_tenant_host"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/hosts/{host}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["console_remove_tenant_host"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/hosts/{host}/primary": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["console_set_primary_tenant_host"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/modules": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["console_list_tenant_modules"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/modules/{key}/data-export": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["console_export_tenant_module_data"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/modules/{key}/data-purge": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["console_purge_tenant_module_data"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/modules/{key}/disable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["console_disable_tenant_module"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/modules/{key}/enable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["console_enable_tenant_module"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/modules/{key}/settings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put: operations["console_update_tenant_module_settings"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/plugins": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["console_list_tenant_plugins"];
        put?: never;
        /** @description Installs or upgrades a plugin in the organization from an `.aphelplugin` sent as the multipart field `package`, with the same checks and acknowledgements as the organization API. With `dry_run=true` nothing is installed and the response describes what the package asks for. Works even when the organization's plugin management is switched off. */
        post: operations["console_install_tenant_plugin"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/plugins/packs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Installs or upgrades a plugin pack (`.aphelpack`, multipart field `package`) in the organization. With `dry_run=true` nothing is installed and the response describes the pack. Status 207 means some plugins failed. */
        post: operations["console_install_tenant_plugin_pack"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/plugins/readiness": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["console_tenant_plugins_readiness"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/plugins/{key}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["console_uninstall_tenant_plugin"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/plugins/{key}/disable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["console_disable_tenant_plugin"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/plugins/{key}/enable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["console_enable_tenant_plugin"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/plugins/{key}/grants": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["console_reapply_tenant_plugin_grants"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/plugins/{key}/settings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["console_tenant_plugin_settings"];
        put: operations["console_update_tenant_plugin_settings"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/resume": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["console_resume_tenant"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/tenants/{slug}/suspend": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["console_suspend_tenant"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/updates": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Update service availability, installed version, latest stable release, operation and history */
        get: operations["console_updates_status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/updates/apply": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Confirm a prepared update; the external service backs up data, restarts the installation and records the outcome */
        post: operations["console_updates_apply"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/updates/check": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Check the latest stable release without changing the running application */
        post: operations["console_updates_check"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/console/v1/updates/prepare": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Download and verify the latest stable image; does not restart the application */
        post: operations["console_updates_prepare"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        AcceptInvitationRequest: {
            password: string;
        };
        ApplySystemUpdate: {
            confirm: boolean;
            operation_id: string;
        };
        AuditEntryResponse: {
            action: string;
            details: Record<string, never>;
            id: string;
            ip?: string | null;
            /** Format: date-time */
            occurred_at: string;
            operator_email?: string | null;
            tenant_id?: string | null;
        };
        CatalogModuleResponse: {
            dependencies: string[];
            description: string;
            key: string;
            name: string;
            requires_configuration: boolean;
        };
        CatalogResponse: {
            blueprints: components["schemas"]["InstallerProfile"][];
            export_available: boolean;
            modules: components["schemas"]["CatalogModuleResponse"][];
            server_public_ip?: string | null;
        };
        ChangePasswordRequest: {
            current_password: string;
            new_password: string;
        };
        ConfiguredTenantRequest: {
            admin_email: string;
            admin_name: string;
            admin_password: string;
            blueprint?: string | null;
            /** @example BRL */
            currency: string;
            /** @example acme.example.com */
            host: string;
            /** @example pt-BR */
            locale: string;
            modules?: string[];
            name: string;
            /** @example acme */
            slug: string;
            /** @example America/Sao_Paulo */
            timezone: string;
        };
        CreateTenantRequest: {
            host: string;
            /** @enum {string} */
            mode: "blank";
        } | (components["schemas"]["ConfiguredTenantRequest"] & {
            /** @enum {string} */
            mode: "configured";
        });
        Data_CatalogResponse: {
            data: {
                blueprints: components["schemas"]["InstallerProfile"][];
                export_available: boolean;
                modules: components["schemas"]["CatalogModuleResponse"][];
                server_public_ip?: string | null;
            };
        };
        Data_InvitationDetailsResponse: {
            data: {
                display_name: string;
                email: string;
            };
        };
        Data_InvitationResponse: {
            data: {
                invite_url: string;
                operator: components["schemas"]["OperatorResponse"];
            };
        };
        Data_ModuleDataExportResponse: {
            data: {
                data: Record<string, never>;
                exported_at: string;
                key: string;
                version: string;
            };
        };
        Data_ModuleResponse: {
            data: {
                can_test: boolean;
                compatibility: components["schemas"]["ModuleCompatibilityResponse"];
                configured_secrets: string[];
                data_exported_at?: string | null;
                data_purged_at?: string | null;
                dependencies: components["schemas"]["ModuleDependencyResponse"][];
                dependents: components["schemas"]["ModuleLinkResponse"][];
                description: string;
                disabled_at?: string | null;
                enabled: boolean;
                enabled_at?: string | null;
                enabled_dependents: string[];
                events: components["schemas"]["ModuleEventResponse"][];
                extension_dependents: string[];
                has_setup_hints: boolean;
                held_by?: string | null;
                installed_version?: string | null;
                /** @example proposals */
                key: string;
                min_aphel_version: string;
                name: string;
                optional_dependencies: components["schemas"]["ModuleLinkResponse"][];
                permissions: components["schemas"]["ModulePermissionResponse"][];
                required: boolean;
                settings: Record<string, never>;
                settings_schema: components["schemas"]["ModuleSettingFieldResponse"][];
                update_available: boolean;
                /** @example 0.1.0 */
                version: string;
            };
        };
        Data_OperatorResponse: {
            data: {
                /** Format: date-time */
                created_at: string;
                display_name: string;
                email: string;
                id: string;
                /** Format: date-time */
                last_login_at?: string | null;
                /** @example active */
                status: string;
            };
        };
        Data_PluginPayload: {
            data: Record<string, never>;
        };
        Data_SystemUpdateStatus: {
            data: {
                checked_at?: string | null;
                current_version: string;
                enabled: boolean;
                history: components["schemas"]["SystemUpdateOperation"][];
                latest?: null | components["schemas"]["SystemRelease"];
                managed_by_console: boolean;
                operation?: null | components["schemas"]["SystemUpdateOperation"];
            };
        };
        Data_TenantHealthResponse: {
            data: {
                available: boolean;
                error?: string | null;
                /** Format: int64 */
                oldest_ready_age_seconds?: number | null;
                /** Format: int64 */
                pending_migrations: number;
                plugin_backend_errors: string[];
                queue: components["schemas"]["QueueTotalsResponse"];
                /** Format: int64 */
                recent_failures: number;
                workers: components["schemas"]["WorkerResponse"][];
            };
        };
        Data_TenantResponse: {
            data: {
                /** Format: date-time */
                created_at: string;
                /** Format: int64 */
                database_bytes?: number | null;
                database_name: string;
                hosts: components["schemas"]["TenantHostResponse"][];
                id: string;
                last_error?: string | null;
                /** Format: date-time */
                last_write_at?: string | null;
                /** Format: date-time */
                migrated_at?: string | null;
                name: string;
                plugin_management: boolean;
                slug: string;
                /** @example active */
                status: string;
            };
        };
        Data_Vec_ModuleResponse: {
            data: {
                can_test: boolean;
                compatibility: components["schemas"]["ModuleCompatibilityResponse"];
                configured_secrets: string[];
                data_exported_at?: string | null;
                data_purged_at?: string | null;
                dependencies: components["schemas"]["ModuleDependencyResponse"][];
                dependents: components["schemas"]["ModuleLinkResponse"][];
                description: string;
                disabled_at?: string | null;
                enabled: boolean;
                enabled_at?: string | null;
                enabled_dependents: string[];
                events: components["schemas"]["ModuleEventResponse"][];
                extension_dependents: string[];
                has_setup_hints: boolean;
                held_by?: string | null;
                installed_version?: string | null;
                /** @example proposals */
                key: string;
                min_aphel_version: string;
                name: string;
                optional_dependencies: components["schemas"]["ModuleLinkResponse"][];
                permissions: components["schemas"]["ModulePermissionResponse"][];
                required: boolean;
                settings: Record<string, never>;
                settings_schema: components["schemas"]["ModuleSettingFieldResponse"][];
                update_available: boolean;
                /** @example 0.1.0 */
                version: string;
            }[];
        };
        Data_Vec_OperatorResponse: {
            data: {
                /** Format: date-time */
                created_at: string;
                display_name: string;
                email: string;
                id: string;
                /** Format: date-time */
                last_login_at?: string | null;
                /** @example active */
                status: string;
            }[];
        };
        Data_Vec_TenantResponse: {
            data: {
                /** Format: date-time */
                created_at: string;
                /** Format: int64 */
                database_bytes?: number | null;
                database_name: string;
                hosts: components["schemas"]["TenantHostResponse"][];
                id: string;
                last_error?: string | null;
                /** Format: date-time */
                last_write_at?: string | null;
                /** Format: date-time */
                migrated_at?: string | null;
                name: string;
                plugin_management: boolean;
                slug: string;
                /** @example active */
                status: string;
            }[];
        };
        DeleteTenantRequest: {
            confirm: string;
            export: boolean;
        };
        ErrorBody: {
            /** @example validation_error */
            code: string;
            details?: Record<string, never> | null;
            message: string;
        };
        ErrorEnvelope: {
            error: components["schemas"]["ErrorBody"];
        };
        HostRequest: {
            /** @example app.acme.example.com */
            host: string;
        };
        InstallerProfile: {
            key: string;
            modules: string[];
            required: string[];
        };
        InvitationDetailsResponse: {
            display_name: string;
            email: string;
        };
        InvitationResponse: {
            invite_url: string;
            operator: components["schemas"]["OperatorResponse"];
        };
        InviteOperatorRequest: {
            display_name: string;
            email: string;
        };
        ModuleCompatibilityResponse: {
            detail?: string | null;
            /** @example compatible */
            status: string;
        };
        ModuleDataExportResponse: {
            data: Record<string, never>;
            exported_at: string;
            key: string;
            version: string;
        };
        ModuleDependencyResponse: {
            enabled: boolean;
            installed: boolean;
            key: string;
            requirement: string;
        };
        ModuleEventResponse: {
            description: string;
            event_type: string;
        };
        ModuleLinkResponse: {
            enabled: boolean;
            key: string;
            optional: boolean;
        };
        ModulePermissionResponse: {
            label: string;
            permission: string;
            record_scoped: boolean;
        };
        ModuleResponse: {
            can_test: boolean;
            compatibility: components["schemas"]["ModuleCompatibilityResponse"];
            configured_secrets: string[];
            data_exported_at?: string | null;
            data_purged_at?: string | null;
            dependencies: components["schemas"]["ModuleDependencyResponse"][];
            dependents: components["schemas"]["ModuleLinkResponse"][];
            description: string;
            disabled_at?: string | null;
            enabled: boolean;
            enabled_at?: string | null;
            enabled_dependents: string[];
            events: components["schemas"]["ModuleEventResponse"][];
            extension_dependents: string[];
            has_setup_hints: boolean;
            held_by?: string | null;
            installed_version?: string | null;
            /** @example proposals */
            key: string;
            min_aphel_version: string;
            name: string;
            optional_dependencies: components["schemas"]["ModuleLinkResponse"][];
            permissions: components["schemas"]["ModulePermissionResponse"][];
            required: boolean;
            settings: Record<string, never>;
            settings_schema: components["schemas"]["ModuleSettingFieldResponse"][];
            update_available: boolean;
            /** @example 0.1.0 */
            version: string;
        };
        ModuleSettingFieldResponse: {
            default?: Record<string, never> | null;
            field_types: string[];
            help?: string | null;
            key: string;
            label: string;
            /** Format: int64 */
            max?: number | null;
            /** Format: int64 */
            min?: number | null;
            options: components["schemas"]["ModuleSettingOptionResponse"][];
            record_type?: string | null;
            required: boolean;
            /** @example text */
            type: string;
        };
        ModuleSettingOptionResponse: {
            label: string;
            value: string;
        };
        OperatorResponse: {
            /** Format: date-time */
            created_at: string;
            display_name: string;
            email: string;
            id: string;
            /** Format: date-time */
            last_login_at?: string | null;
            /** @example active */
            status: string;
        };
        /**
         * @example {
         *       "page": 1,
         *       "per_page": 25,
         *       "total": 120
         *     }
         */
        PageMeta: {
            /** Format: int32 */
            page: number;
            /** Format: int32 */
            per_page: number;
            /** Format: int64 */
            total: number;
        };
        Page_AuditEntryResponse: {
            data: {
                action: string;
                details: Record<string, never>;
                id: string;
                ip?: string | null;
                /** Format: date-time */
                occurred_at: string;
                operator_email?: string | null;
                tenant_id?: string | null;
            }[];
            meta: components["schemas"]["PageMeta"];
        };
        PluginPayload: Record<string, never>;
        PurgeModuleDataRequest: {
            /** @example proposals */
            confirm: string;
        };
        QueueTotalsResponse: {
            /** Format: int64 */
            failed: number;
            /** Format: int64 */
            ready: number;
            /** Format: int64 */
            running: number;
            /** Format: int64 */
            scheduled: number;
        };
        SignInRequest: {
            email: string;
            password: string;
        };
        SystemRelease: {
            notes: string;
            url: string;
            version: string;
        };
        SystemUpdateOperation: {
            actor: string;
            backup_id?: string | null;
            finished_at?: string | null;
            id: string;
            message_code?: string | null;
            phase: string;
            started_at: string;
            version: string;
        };
        SystemUpdateStatus: {
            checked_at?: string | null;
            current_version: string;
            enabled: boolean;
            history: components["schemas"]["SystemUpdateOperation"][];
            latest?: null | components["schemas"]["SystemRelease"];
            managed_by_console: boolean;
            operation?: null | components["schemas"]["SystemUpdateOperation"];
        };
        TenantFeaturesRequest: {
            plugin_management: boolean;
        };
        TenantHealthResponse: {
            available: boolean;
            error?: string | null;
            /** Format: int64 */
            oldest_ready_age_seconds?: number | null;
            /** Format: int64 */
            pending_migrations: number;
            plugin_backend_errors: string[];
            queue: components["schemas"]["QueueTotalsResponse"];
            /** Format: int64 */
            recent_failures: number;
            workers: components["schemas"]["WorkerResponse"][];
        };
        TenantHostResponse: {
            host: string;
            is_primary: boolean;
        };
        TenantResponse: {
            /** Format: date-time */
            created_at: string;
            /** Format: int64 */
            database_bytes?: number | null;
            database_name: string;
            hosts: components["schemas"]["TenantHostResponse"][];
            id: string;
            last_error?: string | null;
            /** Format: date-time */
            last_write_at?: string | null;
            /** Format: date-time */
            migrated_at?: string | null;
            name: string;
            plugin_management: boolean;
            slug: string;
            /** @example active */
            status: string;
        };
        UpdateModuleSettingsRequest: {
            settings: Record<string, never>;
        };
        WorkerResponse: {
            hostname?: string | null;
            instance_id: string;
            /** Format: date-time */
            last_seen_at: string;
            version: string;
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    console_audit: {
        parameters: {
            query?: {
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Operator actions, newest first */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_AuditEntryResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_catalog: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Optional modules and blueprints a new tenant can start with, and whether tenants can be exported */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CatalogResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_invitation: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Token from the invitation link */
                token: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Who the invitation is for */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_InvitationDetailsResponse"];
                };
            };
            /** @description Invalid or expired link (`operator_invitation_invalid`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_accept_invitation: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Token from the invitation link */
                token: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AcceptInvitationRequest"];
            };
        };
        responses: {
            /** @description Password chosen, operator active and signed in */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_OperatorResponse"];
                };
            };
            /** @description Invalid or expired link (`operator_invitation_invalid`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid password (`operator_password_invalid`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_list_operators: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Every operator, deactivated ones last */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_OperatorResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_invite_operator: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["InviteOperatorRequest"];
            };
        };
        responses: {
            /** @description Operator invited; `invite_url` is shown once and valid for 7 days */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_InvitationResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Email already used (`operator_email_taken`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid email (`operator_email_invalid`) or name (`operator_name_invalid`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_deactivate_operator: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Operator id */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Operator deactivated and signed out everywhere */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_OperatorResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown or already deactivated operator (`operator_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Operators cannot deactivate themselves (`operator_self_deactivation`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_renew_invitation: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Operator id */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description A new invitation link; the previous one stops working */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_InvitationResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The operator is not invited (`operator_not_invited`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_session: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description The signed-in operator */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_OperatorResponse"];
                };
            };
            /** @description No console session (`console_session_required`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_sign_in: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SignInRequest"];
            };
        };
        responses: {
            /** @description Signed in; sets the `aphel_console_session` cookie */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_OperatorResponse"];
                };
            };
            /** @description Wrong email or password (`console_invalid_credentials`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Too many attempts (`rate_limited`) or the account is locked for a few minutes (`console_sign_in_locked`) */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_sign_out: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Signed out; clears the cookie */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    console_change_password: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ChangePasswordRequest"];
            };
        };
        responses: {
            /** @description Password changed; the operator's other sessions are revoked */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Wrong current password (`operator_password_wrong`) or invalid new password (`operator_password_invalid`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_list_tenants: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Every tenant with its hosts, state, database size and last write */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_TenantResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_create_tenant: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateTenantRequest"];
            };
        };
        responses: {
            /** @description Tenant registered as `provisioning`; the database, migrations and installation run in the background and the tenant turns `active`, or keeps `last_error` */
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_TenantResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Slug or host already used (`tenant_slug_taken`, `tenant_host_taken`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid slug, name, host or administrator password (`tenant_slug_invalid`, `tenant_name_invalid`, `tenant_host_invalid`, `operator_password_invalid`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_tenant: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description The tenant */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_TenantResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown tenant (`tenant_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_delete_tenant: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DeleteTenantRequest"];
            };
        };
        responses: {
            /** @description Tenant marked `deleting`; the export (into TENANT_EXPORT_DIR), file removal and database drop run in the background */
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_TenantResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown tenant (`tenant_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description `confirm` does not repeat the slug (`tenant_deletion_unconfirmed`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Export asked but this build cannot export (`configuration_error`) */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_update_tenant_features: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TenantFeaturesRequest"];
            };
        };
        responses: {
            /** @description Features saved. With `plugin_management` off, the organization cannot open the plugins screen or install, enable, disable, configure or remove plugins (`plugin_management_disabled`); enabled plugins keep running */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_TenantResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown tenant (`tenant_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_tenant_health: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Jobs, workers, pending migrations, plugin backend errors and recent failures of an active tenant; `available` is false otherwise */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_TenantHealthResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown tenant (`tenant_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_add_tenant_host: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["HostRequest"];
            };
        };
        responses: {
            /** @description Host added */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_TenantResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Host already used (`tenant_host_taken`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid host (`tenant_host_invalid`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_remove_tenant_host: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
                /** @description Host to remove */
                host: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Host removed; the next host becomes primary when the primary is removed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_TenantResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown host (`tenant_host_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description It is the only host (`tenant_last_host`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_set_primary_tenant_host: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
                /** @description Host to make primary */
                host: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Primary host changed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_TenantResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown host (`tenant_host_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_list_tenant_modules: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Modules compiled into this build and their state in the organization */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_ModuleResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown tenant (`tenant_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_export_tenant_module_data: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
                /** @description Module key */
                key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description The exported data */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ModuleDataExportResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown module (`module_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_purge_tenant_module_data: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
                /** @description Module key */
                key: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PurgeModuleDataRequest"];
            };
        };
        responses: {
            /** @description Data removed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ModuleResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown module (`module_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The module is enabled or was not exported after being disabled */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing confirmation (`module_confirmation_required`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_disable_tenant_module: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
                /** @description Module key */
                key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Module disabled */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ModuleResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown module (`module_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description An enabled module depends on it (`module_required_by`) or the build requires it (`module_required`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_enable_tenant_module: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
                /** @description Module key */
                key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Module enabled */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ModuleResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown module (`module_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Incompatible module or a dependency is disabled */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Required settings are missing (`module_settings_required`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_update_tenant_module_settings: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
                /** @description Module key */
                key: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateModuleSettingsRequest"];
            };
        };
        responses: {
            /** @description Saved */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ModuleResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown module (`module_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid settings */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_list_tenant_plugins: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Runtime plugins installed in the organization, in the shape of the organization API (`PluginResponse`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PluginPayload"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown tenant (`tenant_not_found`) or no plugin module in this build (`module_not_installed`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The organization is not available (`tenant_unavailable`) */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_install_tenant_plugin: {
        parameters: {
            query?: {
                dry_run?: boolean;
                acknowledge_conflicts?: boolean;
                acknowledge_web_version?: string | null;
            };
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
            };
            cookie?: never;
        };
        /** @description Field `package` with the .aphelplugin file */
        requestBody?: {
            content: {
                "multipart/form-data": unknown;
            };
        };
        responses: {
            /** @description Dry run: what the package declares and asks for (`PluginConsentResponse`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PluginPayload"];
                };
            };
            /** @description Plugin installed or upgraded (`PluginResponse`) */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PluginPayload"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Conflicts or web code not acknowledged */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid package */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_install_tenant_plugin_pack: {
        parameters: {
            query?: {
                dry_run?: boolean;
                acknowledge_conflicts?: boolean;
                acknowledge_web_version?: string | null;
            };
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
            };
            cookie?: never;
        };
        /** @description Field `package` with the .aphelpack file */
        requestBody?: {
            content: {
                "multipart/form-data": unknown;
            };
        };
        responses: {
            /** @description Dry run: what the pack declares and asks for */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PluginPayload"];
                };
            };
            /** @description Pack installed */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PluginPayload"];
                };
            };
            /** @description Some plugins of the pack failed */
            207: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PluginPayload"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Conflicts or web code not acknowledged */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid pack */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_tenant_plugins_readiness: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Plugin catalog readiness (`loading`, `ready` or `degraded`) and backend errors */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PluginPayload"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_uninstall_tenant_plugin: {
        parameters: {
            query?: {
                purge?: boolean;
            };
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
                /** @description Plugin key */
                key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Plugin removed; with `purge=true` its records and fields are deleted too */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Plugin not installed (`plugin_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_disable_tenant_plugin: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
                /** @description Plugin key */
                key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Plugin disabled (`PluginResponse`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PluginPayload"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Plugin not installed (`plugin_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_enable_tenant_plugin: {
        parameters: {
            query?: {
                acknowledge_conflicts?: boolean;
                acknowledge_web_version?: string | null;
            };
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
                /** @description Plugin key */
                key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Plugin enabled (`PluginResponse`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PluginPayload"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Plugin not installed (`plugin_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Web code or overlaps not acknowledged */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_reapply_tenant_plugin_grants: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
                /** @description Plugin key */
                key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Grants reapplied to the roles that exist now (`PluginResponse`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PluginPayload"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Plugin not installed (`plugin_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_tenant_plugin_settings: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
                /** @description Plugin key */
                key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Settings with their current values (`PluginSettingResponse[]`); secrets only report `configured` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PluginPayload"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Plugin not installed (`plugin_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_update_tenant_plugin_settings: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
                /** @description Plugin key */
                key: string;
            };
            cookie?: never;
        };
        /** @description Map of setting key to value */
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description Settings saved (`PluginResponse`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PluginPayload"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Plugin not installed (`plugin_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid settings (`plugin_settings_invalid`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_resume_tenant: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Tenant served again */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_TenantResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The tenant is not suspended (`tenant_status_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_suspend_tenant: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tenant slug */
                slug: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Tenant suspended: its hosts answer 503 and its jobs stop */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_TenantResponse"];
                };
            };
            /** @description No console session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The tenant is not active (`tenant_status_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_updates_status: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Update status */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_SystemUpdateStatus"];
                };
            };
            /** @description Authentication required */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Not allowed in this context */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Update state changed or another operation is running */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description External update service unavailable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_updates_apply: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ApplySystemUpdate"];
            };
        };
        responses: {
            /** @description Update status */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_SystemUpdateStatus"];
                };
            };
            /** @description Authentication required */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Not allowed in this context */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Update state changed or another operation is running */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description External update service unavailable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_updates_check: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Update status */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_SystemUpdateStatus"];
                };
            };
            /** @description Authentication required */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Not allowed in this context */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Update state changed or another operation is running */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description External update service unavailable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    console_updates_prepare: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Update status */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_SystemUpdateStatus"];
                };
            };
            /** @description Authentication required */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Not allowed in this context */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Update state changed or another operation is running */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description External update service unavailable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
}
