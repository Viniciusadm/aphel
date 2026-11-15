export interface paths {
    "/api/v1/reminders": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["reminders_list"];
        put?: never;
        post: operations["reminders_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/reminders/{reminder_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["reminders_get"];
        put?: never;
        post?: never;
        delete: operations["reminders_delete"];
        options?: never;
        head?: never;
        patch: operations["reminders_update"];
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        ReminderResponse: {
            created_at: string;
            id: string;
            note?: string | null;
            record?: null | components["schemas"]["LinkedRecordResponse"];
            record_id?: string | null;
            record_type?: string | null;
            /** @example 2026-10-22T13:00:00Z */
            remind_at: string;
            sent_at?: string | null;
            status: components["schemas"]["ReminderStatus"];
            updated_at: string;
        };
        /** @enum {string} */
        ReminderStatus: "pending" | "sent" | "dismissed";
        LinkedRecordResponse: {
            record_id: string;
            record_type: string;
            title?: string | null;
        };
        UpdateReminderRequest: {
            note?: string | null;
            record_id?: string | null;
            record_type?: string | null;
            remind_at?: string | null;
            status?: null | components["schemas"]["ReminderStatus"];
        };
        Data_ReminderResponse: {
            data: {
                created_at: string;
                id: string;
                note?: string | null;
                record?: null | components["schemas"]["LinkedRecordResponse"];
                record_id?: string | null;
                record_type?: string | null;
                /** @example 2026-10-22T13:00:00Z */
                remind_at: string;
                sent_at?: string | null;
                status: components["schemas"]["ReminderStatus"];
                updated_at: string;
            };
        };
        CreateReminderRequest: {
            /** @example Follow up on the proposal */
            note?: string | null;
            record_id?: string | null;
            /** @example deals.deal */
            record_type?: string | null;
            /** @example 2026-10-22T13:00:00Z */
            remind_at: string;
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
        Page_ReminderResponse: {
            data: {
                created_at: string;
                id: string;
                note?: string | null;
                record?: null | components["schemas"]["LinkedRecordResponse"];
                record_id?: string | null;
                record_type?: string | null;
                /** @example 2026-10-22T13:00:00Z */
                remind_at: string;
                sent_at?: string | null;
                status: components["schemas"]["ReminderStatus"];
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
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    reminders_list: {
        parameters: {
            query?: {
                /** @example pending */
                status?: string;
                /** @example deals.deal */
                record_type?: string;
                record_id?: string;
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description The user's own reminders (reminders are personal), sorted by `remind_at`. Filters: `status` (`pending`, `sent`, `dismissed`) and `record_type` + `record_id` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_ReminderResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid filter or pagination (`validation_error`) */
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
    reminders_create: {
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
                "application/json": components["schemas"]["CreateReminderRequest"];
            };
        };
        responses: {
            /** @description Personal reminder created as `pending`; at `remind_at` the user gets a `reminder` notification (in-app and e-mail, following the notification preferences). A reminder is linked to a record (`record_type` + `record_id`, which the user must be able to read) or has a `note` */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ReminderResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
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
            /** @description Invalid fields or unreadable record (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    reminders_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Reminder id */
                reminder_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description One of the user's reminders */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ReminderResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Reminder not found or owned by someone else (`reminder_not_found`) */
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
    reminders_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Reminder id */
                reminder_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Reminder deleted */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Reminder not found (`reminder_not_found`) */
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
    reminders_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the reminder as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict` */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Reminder id */
                reminder_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateReminderRequest"];
            };
        };
        responses: {
            /** @description Reminder updated; only the sent fields change. A new `remind_at` or `status=pending` re-arms it; `status=dismissed` silences it */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ReminderResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Reminder not found (`reminder_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The reminder changed after the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid fields or unreadable record (`validation_error`) */
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
}
