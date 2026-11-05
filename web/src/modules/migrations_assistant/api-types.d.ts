export interface paths {
    "/api/v1/migration-assistant/history": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["migration_assistant_history"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/migration-assistant/imports": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["migration_assistant_create_import"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/migration-assistant/preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["migration_assistant_preview"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/migration-assistant/sources": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["migration_assistant_sources"];
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
        MigrationImportResponse: {
            import: components["schemas"]["ImportJobResponse"];
            preview: components["schemas"]["MigrationPreviewResponse"];
        };
        MigrationPreviewResponse: {
            /** Format: int64 */
            changed_cells: number;
            /**
             * @example {
             *       "stage_id": [
             *         "Qualified",
             *         "Proposal Made"
             *       ]
             *     }
             */
            distinct: {
                [key: string]: string[];
            };
            headers: string[];
            /**
             * @example {
             *       "owner_id": 2
             *     }
             */
            issues: {
                [key: string]: number;
            };
            recognized: components["schemas"]["RecognizedColumnResponse"][];
            record_type: string;
            /** Format: int64 */
            row_count: number;
            sample: string[][];
            source: string;
            step: string;
            unrecognized: string[];
        };
        RecognizedColumnResponse: {
            /** @example Deal - Value */
            header: string;
            index: number;
            /** @example amount */
            target: string;
            /** @example money */
            transform: string;
        };
        ImportJobResponse: {
            created_at: string;
            created_by: string;
            /** @example ; */
            delimiter: string;
            duplicate_keys: components["schemas"]["DuplicateKeyDto"][];
            /** @example utf-8 */
            encoding: string;
            error_file_id?: string | null;
            failure_reason?: string | null;
            file_id?: string | null;
            file_name: string;
            finished_at?: string | null;
            headers: string[];
            id: string;
            mapping: (string | null)[];
            options: components["schemas"]["ImportOptionsDto"];
            preview: components["schemas"]["ImportPreviewRow"][];
            privacy_redacted_at?: string | null;
            /** @example people.person */
            record_type: string;
            retry_of?: string | null;
            /** Format: int32 */
            row_count: number;
            started_at?: string | null;
            /** @example draft */
            status: string;
            targets: components["schemas"]["ImportTargetResponse"][];
            totals: components["schemas"]["ImportTotalsResponse"];
            updated_at: string;
        };
        ImportTotalsResponse: {
            /** Format: int64 */
            failed: number;
            /** Format: int64 */
            imported: number;
            /** Format: int64 */
            pending: number;
            /** Format: int64 */
            skipped: number;
            /** Format: int64 */
            total: number;
            /** Format: int64 */
            updated: number;
        };
        ImportTargetResponse: {
            can_create_reference: boolean;
            /** @example email */
            field_type: string;
            is_custom: boolean;
            /** @example email */
            key: string;
            /** @example Email */
            label: string;
            multiple: boolean;
            /** @example organizations.organization */
            relation_target?: string | null;
            required: boolean;
            /** @example name */
            satisfied_by?: string | null;
        };
        ImportPreviewRow: {
            cells: string[];
            /** Format: int32 */
            row_index: number;
        };
        ImportOptionsDto: {
            create_missing_references: boolean;
            distribution_rule_id?: string | null;
            duplicate_key: components["schemas"]["DuplicateKeyDto"];
            duplicate_strategy: components["schemas"]["DuplicateStrategyDto"];
        };
        /** @enum {string} */
        DuplicateStrategyDto: "skip" | "update" | "create";
        /** @enum {string} */
        DuplicateKeyDto: "none" | "email" | "phone" | "domain" | "name";
        MigrationSourceResponse: {
            /** @example month_first */
            date_order: string;
            /** @example dot */
            decimal: string;
            /** @example pipedrive */
            key: string;
            /** @example Pipedrive */
            name: string;
            steps: components["schemas"]["MigrationStepResponse"][];
        };
        MigrationStepResponse: {
            columns: components["schemas"]["MigrationColumnResponse"][];
            /** @example deals.csv */
            file_hint: string;
            /** @example deals */
            key: string;
            /** @example import */
            kind: string;
            links: string[];
            /** @example deals.deal */
            record_type: string;
        };
        MigrationColumnResponse: {
            /**
             * @example [
             *       "Deal - Value",
             *       "Negócio - Valor"
             *     ]
             */
            headers: string[];
            /** @example amount */
            target: string;
            /** @example money */
            transform: string;
        };
        Data_Vec_MigrationSourceResponse: {
            data: {
                /** @example month_first */
                date_order: string;
                /** @example dot */
                decimal: string;
                /** @example pipedrive */
                key: string;
                /** @example Pipedrive */
                name: string;
                steps: components["schemas"]["MigrationStepResponse"][];
            }[];
        };
        Data_MigrationPreviewResponse: {
            data: {
                /** Format: int64 */
                changed_cells: number;
                /**
                 * @example {
                 *       "stage_id": [
                 *         "Qualified",
                 *         "Proposal Made"
                 *       ]
                 *     }
                 */
                distinct: {
                    [key: string]: string[];
                };
                headers: string[];
                /**
                 * @example {
                 *       "owner_id": 2
                 *     }
                 */
                issues: {
                    [key: string]: number;
                };
                recognized: components["schemas"]["RecognizedColumnResponse"][];
                record_type: string;
                /** Format: int64 */
                row_count: number;
                sample: string[][];
                source: string;
                step: string;
                unrecognized: string[];
            };
        };
        Data_MigrationImportResponse: {
            data: {
                import: components["schemas"]["ImportJobResponse"];
                preview: components["schemas"]["MigrationPreviewResponse"];
            };
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
        Data_MigrationHistoryResponse: {
            data: {
                preview: components["schemas"]["MigrationPreviewResponse"];
                report: components["schemas"]["HistoryReportResponse"];
            };
        };
        HistoryReportResponse: {
            /** Format: int64 */
            created: number;
            errors: components["schemas"]["HistoryRowErrorResponse"][];
            /** Format: int64 */
            failed: number;
            /** Format: int64 */
            skipped: number;
            /** Format: int64 */
            total: number;
        };
        HistoryRowErrorResponse: {
            message: string;
            /** @example 12 */
            row: number;
        };
        MigrationHistoryResponse: {
            preview: components["schemas"]["MigrationPreviewResponse"];
            report: components["schemas"]["HistoryReportResponse"];
        };
        MigrationUploadForm: {
            /** Format: binary */
            file: string;
            /** @example {"stage_id":{"Qualified":"Qualification"},"owner_id":{"Ana Souza":"ana@company.com"}} */
            maps?: string | null;
            /** @example pipedrive */
            source: string;
            /** @example deals */
            step: string;
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
    migration_assistant_history: {
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
                "multipart/form-data": components["schemas"]["MigrationUploadForm"];
            };
        };
        responses: {
            /** @description Migrates the tasks or notes exported by the source tool (steps `tasks` and `notes`, up to 5 000 rows per file) right away through the tasks and notes use cases (validation, permission, event and audit per row). Each row is linked to the deal (by title), contact (by e-mail or name) and company (by name) already in Aphel, so import organizations, people and deals first; activity types are matched by name and owners by name or e-mail, done tasks keep their due date as completion. Notes need a linked record. Attachments are not part of the CSV exports and are not migrated. Rows that fail are listed in `report.errors` (first 100). Requires `tasks.create` or `notes.create` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_MigrationHistoryResponse"];
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
            /** @description Missing `tasks.create` or `notes.create` (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown source or step, or the tasks or notes module is disabled (`migration_source_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The file exceeds the upload limit of the organization settings (`file_too_large`) */
            413: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid form, a organizations/people/deals step (`migration_history_step`), empty or invalid file or more than 5 000 rows (`empty_import`, `invalid_csv`, `too_many_rows`, `too_many_columns`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    migration_assistant_create_import: {
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
                "multipart/form-data": components["schemas"]["MigrationUploadForm"];
            };
        };
        responses: {
            /** @description Converts the CSV of the source tool (dates, money, phones, owners, deal status and the `maps` of stages, workflow and owners), stores it as a draft import (`POST /imports`) and pre-fills its mapping and options (duplicates by the step key, missing organizations created by name). Review and start it with the import endpoints (`/imports/{import_id}`). Requires `<type>.create` */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_MigrationImportResponse"];
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
            /** @description Missing `<type>.create` (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown source or step (`migration_source_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The file exceeds the upload limit of the organization settings (`file_too_large`) */
            413: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid form or `maps`, empty or invalid CSV (`empty_import`, `invalid_csv`, `too_many_rows`, `too_many_columns`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    migration_assistant_preview: {
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
                "multipart/form-data": components["schemas"]["MigrationUploadForm"];
            };
        };
        responses: {
            /** @description Reads the CSV exported by the source tool without storing it: recognized and unrecognized columns, cells converted (dates, money, phones, owners by name, deal status), conversion issues per field, distinct values of stage, pipeline, owner and status columns (deal status is read from a status column or from won/lost stage names and clears the stage, so the deal goes to the first won or lost stage of its pipeline) (to build `maps`) and the first normalized rows. Requires `<type>.create` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_MigrationPreviewResponse"];
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
            /** @description Missing `<type>.create` (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown source or step (`migration_source_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The file exceeds the upload limit of the organization settings (`file_too_large`) */
            413: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid form or `maps`, empty or invalid CSV (`empty_import`, `invalid_csv`, `too_many_rows`, `too_many_columns`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    migration_assistant_sources: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Supported source CRMs (Pipedrive, HubSpot, RD Station CRM, Agendor), the suggested import order (organizations, people, deals, then tasks and notes, whose `kind` sends them to `/migration-assistant/history`) and the columns recognized in each export, with the Aphel field and the conversion applied. Headers were inferred from public documentation and may vary by account language and version */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_MigrationSourceResponse"];
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
