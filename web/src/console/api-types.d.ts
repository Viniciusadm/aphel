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
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        AcceptInvitationRequest: {
            password: string;
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
        };
        CatalogResponse: {
            blueprints: components["schemas"]["InstallerProfile"][];
            export_available: boolean;
            modules: components["schemas"]["CatalogModuleResponse"][];
        };
        ChangePasswordRequest: {
            current_password: string;
            new_password: string;
        };
        CreateTenantRequest: {
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
        Data_CatalogResponse: {
            data: {
                blueprints: components["schemas"]["InstallerProfile"][];
                export_available: boolean;
                modules: components["schemas"]["CatalogModuleResponse"][];
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
                slug: string;
                /** @example active */
                status: string;
            };
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
            slug: string;
            /** @example active */
            status: string;
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
}
