export interface paths {
    "/api/v1/installer/restore": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Restores a full backup (`aphel.organization_export` zip) into an empty installation, as an alternative to `POST /installer`: creates the organization and its members (with their passwords), installs the configuration and restores records, relationships, history and files. Available only while the installer is (`GET /installer`). Sign in afterwards with an administrator of the backup. The upload is limited to 2 GiB (use `aphel org import` for larger backups), the archive to 500,000 entries and 64 GiB uncompressed, and the route to 5 attempts per hour per IP address and 20 per hour in total. */
        post: operations["installer_restore"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organization-exports": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["organization_exports_list"];
        put?: never;
        post: operations["organization_exports_request"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organization-exports/{export_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["organization_exports_get"];
        put?: never;
        post?: never;
        delete: operations["organization_exports_delete"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organization-exports/{export_id}/download": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["organization_exports_download"];
        put?: never;
        post?: never;
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
        OrgExportResponse: {
            created_at: string;
            failure_reason?: string | null;
            finished_at?: string | null;
            id: string;
            include_files: boolean;
            requested_by: components["schemas"]["OrgExportRequesterResponse"];
            started_at?: string | null;
            /** @example completed */
            status: string;
            summary?: null | components["schemas"]["OrgExportSummaryResponse"];
            updated_at: string;
        };
        OrgExportSummaryResponse: {
            /** Format: int64 */
            archive_bytes: number;
            /**
             * @example {
             *       "attachments": 3,
             *       "notes": 25
             *     }
             */
            datasets: {
                [key: string]: number;
            };
            /** Format: int64 */
            file_bytes: number;
            /** Format: int64 */
            files: number;
            /** Format: int64 */
            members: number;
            /** Format: int64 */
            missing_files: number;
            /**
             * @example {
             *       "company": 20,
             *       "contact": 60,
             *       "deal": 40
             *     }
             */
            records: {
                [key: string]: number;
            };
        };
        OrgExportRequesterResponse: {
            display_name?: string | null;
            id: string;
        };
        Data_OrgExportResponse: {
            data: {
                created_at: string;
                failure_reason?: string | null;
                finished_at?: string | null;
                id: string;
                include_files: boolean;
                requested_by: components["schemas"]["OrgExportRequesterResponse"];
                started_at?: string | null;
                /** @example completed */
                status: string;
                summary?: null | components["schemas"]["OrgExportSummaryResponse"];
                updated_at: string;
            };
        };
        OrgExportRequest: {
            /** @example true */
            include_files?: boolean | null;
        };
        Page_OrgExportResponse: {
            data: {
                created_at: string;
                failure_reason?: string | null;
                finished_at?: string | null;
                id: string;
                include_files: boolean;
                requested_by: components["schemas"]["OrgExportRequesterResponse"];
                started_at?: string | null;
                /** @example completed */
                status: string;
                summary?: null | components["schemas"]["OrgExportSummaryResponse"];
                updated_at: string;
            }[];
            meta: components["schemas"]["PageMeta"];
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
        ErrorEnvelope: {
            error: components["schemas"]["ErrorBody"];
        };
        ErrorBody: {
            /** @example validation_error */
            code: string;
            details?: Record<string, never> | null;
            message: string;
        };
        Data_InstallerRestoreResponse: {
            data: {
                restored: boolean;
            };
        };
        InstallerRestoreResponse: {
            restored: boolean;
        };
        InstallerRestoreForm: {
            /** Format: binary */
            file: string;
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
    installer_restore: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "multipart/form-data": components["schemas"]["InstallerRestoreForm"];
            };
        };
        responses: {
            /** @description Backup restored */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_InstallerRestoreResponse"];
                };
            };
            /** @description The instance is already installed (`not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The installation is not empty (`organization_not_empty`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The upload is larger than 2 GiB (`payload_too_large`) */
            413: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid form or invalid backup (`invalid_organization_export`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Too many restore attempts (`rate_limited`, with `Retry-After`) */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Database is not configured */
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
    organization_exports_list: {
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
            /** @description Full exports of the organization, most recent first. Requires `organization.export` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_OrgExportResponse"];
                };
            };
            /** @description Missing or invalid session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `organization.export` (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid pagination (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
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
    organization_exports_request: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["OrgExportRequest"];
            };
        };
        responses: {
            /** @description Full export of the organization queued (`organization_export.run`). The worker writes a zip with `manifest.json` (format `aphel.organization_export` v1, Aphel version, source organization, counts and SHA-256 of every entry), the configuration package, one JSON Lines file per record type (including custom objects, with custom fields by stable key), relationships, notes, history and the attached files (`include_files`, default true). Finishes with `organization_export.completed` or `organization_export.failed` and notifies the requester. Requires `organization.export` */
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_OrgExportResponse"];
                };
            };
            /** @description Missing or invalid session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `organization.export` (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Another export of the organization is queued or running (`organization_export_in_progress`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid body (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
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
    organization_exports_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization export id */
                export_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Status and counts of the export. Requires `organization.export` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_OrgExportResponse"];
                };
            };
            /** @description Missing or invalid session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `organization.export` (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Export not found (`organization_export_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
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
    organization_exports_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization export id */
                export_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Export and its zip file deleted; publishes `organization_export.deleted`. Requires `organization.export` */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Missing or invalid session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `organization.export` (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Export not found (`organization_export_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The export is still queued or running (`organization_export_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
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
    organization_exports_download: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization export id */
                export_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description The zip of a completed export, served as an attachment. Requires `organization.export` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/zip": unknown;
                };
            };
            /** @description Missing or invalid session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `organization.export` (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Export not found (`organization_export_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The export is not completed or its file was removed (`organization_export_not_ready`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
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
}
