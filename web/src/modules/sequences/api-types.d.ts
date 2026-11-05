export interface paths {
    "/api/v1/sequence-enrollments": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Enrollments filtered by `contact_id` (needs read access to the contact), `sequence_id` and `status`, newest first. Needs `sequences.read` (own/team/organization over who enrolled). */
        get: operations["sequence_enrollments_list"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/sequence-enrollments/{enrollment_id}/exit": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Removes the contact from the sequence (`sequence.exited` with `exit_reason = removed`). Needs `sequences.enroll`. */
        post: operations["sequence_enrollments_exit"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/sequence-enrollments/{enrollment_id}/pause": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Pauses an active enrollment (`sequence_enrollment.paused`); no step runs until it is resumed. Needs `sequences.enroll` (own/team/organization over who enrolled). */
        post: operations["sequence_enrollments_pause"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/sequence-enrollments/{enrollment_id}/resume": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Resumes a paused enrollment (`sequence_enrollment.resumed`); the pending step runs on the next cycle, respecting the sending window and limits. Needs `sequences.enroll`. */
        post: operations["sequence_enrollments_resume"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/sequences": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Sequences (cadences) of the organization ordered by name, with their steps and enrollment counts. Needs `sequences.read` (counts follow its own/team/organization scope over who enrolled) or `sequences.manage`. */
        get: operations["sequences_list"];
        put?: never;
        /** @description Creates a sequence (`sequence.created`). Steps run in order: `auto_email` (sent from the mailbox of whoever enrolled the contact, inside the sending window and the daily limit of that mailbox, never two on the same day for the same contact), `manual_email` (an `email` task for the enroller with the rendered draft), `call`, `linkedin` and `task` (tasks for the enroller) and `wait` (`wait_days` business days in the organization timezone). E-mail steps take `template_id` (an organization template) or `subject` + `body_html` with `{{contact.first_name}}`-style variables; later e-mails reply in the same conversation. `window_start`/`window_end` are `HH:MM` in the organization timezone (default 08:00–18:00, business days); `daily_limit` is the maximum of sequence e-mails per mailbox per day (default 50). Needs `sequences.manage`. */
        post: operations["sequences_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/sequences/{sequence_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Sequence with steps and enrollment counts. Needs `sequences.read` or `sequences.manage`. */
        get: operations["sequences_get"];
        put?: never;
        post?: never;
        /** @description Deletes a sequence with its enrollments and step history (`sequence.deleted`). E-mails and tasks already created stay. Needs `sequences.manage`. */
        delete: operations["sequences_delete"];
        options?: never;
        head?: never;
        /** @description Updates a sequence (`sequence.updated`); omitted properties stay unchanged and `steps` replaces the whole list (send the `id` of a step to keep it and its metrics). Active enrollments continue from their current position. `is_active: false` holds every enrollment until the sequence is active again. Needs `sequences.manage`. */
        patch: operations["sequences_update"];
        trace?: never;
    };
    "/api/v1/sequences/{sequence_id}/enrollments": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Enrollments of the sequence, newest first. Needs `sequences.read` (own/team/organization over who enrolled). */
        get: operations["sequences_enrollments_list"];
        put?: never;
        /** @description Enrolls a contact (`sequence.enrolled`). E-mails go out from `mailbox_id` (one of the user's mailboxes) or the user's first connected mailbox. The contact leaves the sequence automatically when they reply, book a meeting, a deal of theirs is won or lost, an e-mail bounces, they object to tracking or they are deleted (`sequence.exited` with `exit_reason`); after the last step the enrollment is `completed` (`sequence.completed`). Needs `sequences.enroll` and read access to the contact. */
        post: operations["sequences_enroll"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/sequences/{sequence_id}/metrics": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Enrollment counts and metrics per step: `executed` (steps run), `sent` (e-mails sent or tasks created), `opened` (tracked e-mails opened), `replied` (replies received in the conversation), `bounced`, `failed` and `exits` (enrollments that left the sequence right after the step). Counts follow the `sequences.read` scope over who enrolled. */
        get: operations["sequences_metrics"];
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
        SequenceEnrollmentResponse: {
            completed_at?: string | null;
            contact_email?: string | null;
            contact_id: string;
            contact_name: string;
            created_at: string;
            /** Format: int32 */
            current_position: number;
            enrolled_by: string;
            enrolled_by_name?: string | null;
            /** Format: int32 */
            exit_position?: number | null;
            exit_reason?: null | components["schemas"]["SequenceExitReason"];
            exited_at?: string | null;
            id: string;
            mailbox_id?: string | null;
            next_run_at?: string | null;
            paused_at?: string | null;
            sequence_id: string;
            sequence_name: string;
            source: components["schemas"]["SequenceEnrollmentSource"];
            status: components["schemas"]["SequenceEnrollmentStatus"];
            updated_at: string;
        };
        /** @enum {string} */
        SequenceEnrollmentStatus: "active" | "paused" | "completed" | "exited";
        /** @enum {string} */
        SequenceEnrollmentSource: "manual" | "bulk" | "automation";
        /** @enum {string} */
        SequenceExitReason: "replied" | "meeting_booked" | "deal_won" | "deal_lost" | "bounced" | "objection" | "contact_deleted" | "removed" | "contact_unavailable" | "enroller_unavailable" | "send_failed";
        SequenceMetricsResponse: {
            enrollments: components["schemas"]["SequenceEnrollmentCountsResponse"];
            steps: components["schemas"]["SequenceStepMetricsResponse"][];
        };
        SequenceStepMetricsResponse: {
            /** Format: int64 */
            bounced: number;
            /** Format: int64 */
            executed: number;
            /** Format: int64 */
            exits: number;
            /** Format: int64 */
            failed: number;
            kind?: null | components["schemas"]["SequenceStepKind"];
            /** Format: int64 */
            opened: number;
            /** Format: int32 */
            position: number;
            /** Format: int64 */
            replied: number;
            /** Format: int64 */
            sent: number;
            step_id?: string | null;
        };
        /** @enum {string} */
        SequenceStepKind: "auto_email" | "manual_email" | "call" | "linkedin" | "task" | "wait";
        SequenceEnrollmentCountsResponse: {
            /** Format: int64 */
            active: number;
            /** Format: int64 */
            completed: number;
            /** Format: int64 */
            exited: number;
            /** Format: int64 */
            paused: number;
        };
        SequenceStepRequest: {
            body_html?: string | null;
            /** @example 0192f0c4-8c1e-7d2a-9f3b-1c2d3e4f5a6b */
            id?: string | null;
            kind: components["schemas"]["SequenceStepKind"];
            /** @example Hi {{contact.first_name}} */
            subject?: string | null;
            template_id?: string | null;
            /** @example Call to present the proposal */
            title?: string | null;
            /**
             * Format: int32
             * @example 2
             */
            wait_days?: number | null;
        };
        SequenceResponse: {
            /** @example 2026-10-22T13:45:00Z */
            created_at: string;
            created_by?: string | null;
            /** Format: int32 */
            daily_limit: number;
            description?: string | null;
            enrollments: components["schemas"]["SequenceEnrollmentCountsResponse"];
            id: string;
            is_active: boolean;
            key: string;
            name: string;
            steps: components["schemas"]["SequenceStepResponse"][];
            /** @example 2026-10-22T13:45:00Z */
            updated_at: string;
            updated_by?: string | null;
            /** @example 18:00 */
            window_end: string;
            /** @example 08:00 */
            window_start: string;
        };
        SequenceStepResponse: {
            body_html?: string | null;
            id: string;
            kind: components["schemas"]["SequenceStepKind"];
            /** Format: int32 */
            position: number;
            subject: string;
            template_id?: string | null;
            title: string;
            /** Format: int32 */
            wait_days: number;
        };
        Data_SequenceMetricsResponse: {
            data: {
                enrollments: components["schemas"]["SequenceEnrollmentCountsResponse"];
                steps: components["schemas"]["SequenceStepMetricsResponse"][];
            };
        };
        EnrollContactRequest: {
            contact_id: string;
            mailbox_id?: string | null;
        };
        Data_SequenceResponse: {
            data: {
                /** @example 2026-10-22T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                /** Format: int32 */
                daily_limit: number;
                description?: string | null;
                enrollments: components["schemas"]["SequenceEnrollmentCountsResponse"];
                id: string;
                is_active: boolean;
                key: string;
                name: string;
                steps: components["schemas"]["SequenceStepResponse"][];
                /** @example 2026-10-22T13:45:00Z */
                updated_at: string;
                updated_by?: string | null;
                /** @example 18:00 */
                window_end: string;
                /** @example 08:00 */
                window_start: string;
            };
        };
        SequenceRequest: {
            /**
             * Format: int32
             * @example 50
             */
            daily_limit?: number | null;
            description?: string | null;
            is_active?: boolean | null;
            /** @example outbound_prospecting */
            key?: string | null;
            /** @example Outbound prospecting */
            name?: string | null;
            steps?: components["schemas"]["SequenceStepRequest"][] | null;
            /** @example 18:00 */
            window_end?: string | null;
            /** @example 08:00 */
            window_start?: string | null;
        };
        Page_SequenceResponse: {
            data: {
                /** @example 2026-10-22T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                /** Format: int32 */
                daily_limit: number;
                description?: string | null;
                enrollments: components["schemas"]["SequenceEnrollmentCountsResponse"];
                id: string;
                is_active: boolean;
                key: string;
                name: string;
                steps: components["schemas"]["SequenceStepResponse"][];
                /** @example 2026-10-22T13:45:00Z */
                updated_at: string;
                updated_by?: string | null;
                /** @example 18:00 */
                window_end: string;
                /** @example 08:00 */
                window_start: string;
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
        Data_SequenceEnrollmentResponse: {
            data: {
                completed_at?: string | null;
                contact_email?: string | null;
                contact_id: string;
                contact_name: string;
                created_at: string;
                /** Format: int32 */
                current_position: number;
                enrolled_by: string;
                enrolled_by_name?: string | null;
                /** Format: int32 */
                exit_position?: number | null;
                exit_reason?: null | components["schemas"]["SequenceExitReason"];
                exited_at?: string | null;
                id: string;
                mailbox_id?: string | null;
                next_run_at?: string | null;
                paused_at?: string | null;
                sequence_id: string;
                sequence_name: string;
                source: components["schemas"]["SequenceEnrollmentSource"];
                status: components["schemas"]["SequenceEnrollmentStatus"];
                updated_at: string;
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
        Page_SequenceEnrollmentResponse: {
            data: {
                completed_at?: string | null;
                contact_email?: string | null;
                contact_id: string;
                contact_name: string;
                created_at: string;
                /** Format: int32 */
                current_position: number;
                enrolled_by: string;
                enrolled_by_name?: string | null;
                /** Format: int32 */
                exit_position?: number | null;
                exit_reason?: null | components["schemas"]["SequenceExitReason"];
                exited_at?: string | null;
                id: string;
                mailbox_id?: string | null;
                next_run_at?: string | null;
                paused_at?: string | null;
                sequence_id: string;
                sequence_name: string;
                source: components["schemas"]["SequenceEnrollmentSource"];
                status: components["schemas"]["SequenceEnrollmentStatus"];
                updated_at: string;
            }[];
            meta: components["schemas"]["PageMeta"];
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
    sequence_enrollments_list: {
        parameters: {
            query?: {
                sequence_id?: string;
                contact_id?: string;
                status?: components["schemas"]["SequenceEnrollmentStatus"];
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Enrollments */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_SequenceEnrollmentResponse"];
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
            /** @description Missing `sequences.read` or read access to the contact (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid filters or pagination (`validation_error`) */
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
    sequence_enrollments_exit: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Enrollment id */
                enrollment_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Enrollment exited */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_SequenceEnrollmentResponse"];
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
            /** @description Missing `sequences.enroll` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Enrollment not found (`sequence_enrollment_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The enrollment already finished (`sequence_enrollment_finished`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    sequence_enrollments_pause: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Enrollment id */
                enrollment_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Enrollment paused */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_SequenceEnrollmentResponse"];
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
            /** @description Missing `sequences.enroll` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Enrollment not found (`sequence_enrollment_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The enrollment is not active (`sequence_enrollment_finished`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    sequence_enrollments_resume: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Enrollment id */
                enrollment_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Enrollment resumed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_SequenceEnrollmentResponse"];
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
            /** @description Missing `sequences.enroll` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Enrollment not found (`sequence_enrollment_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The enrollment is not paused (`sequence_enrollment_finished`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    sequences_list: {
        parameters: {
            query?: {
                active?: boolean;
                /** @example prospect */
                search?: string;
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Sequences */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_SequenceResponse"];
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
            /** @description Missing `sequences.read` (`permission_denied`, `insufficient_scope`) */
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
    sequences_create: {
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
                "application/json": components["schemas"]["SequenceRequest"];
            };
        };
        responses: {
            /** @description Sequence created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_SequenceResponse"];
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
            /** @description Missing `sequences.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Another sequence already uses the key (`sequence_key_taken`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid properties or steps (`validation_error` with `details.fields`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    sequences_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Sequence id */
                sequence_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Sequence */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_SequenceResponse"];
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
            /** @description Missing `sequences.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Sequence not found (`sequence_not_found`) */
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
    sequences_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Sequence id */
                sequence_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Sequence deleted */
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
            /** @description Missing `sequences.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Sequence not found (`sequence_not_found`) */
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
    sequences_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the sequence as read (ISO-8601, quotes optional); the update fails with 409 when it changed since */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Sequence id */
                sequence_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SequenceRequest"];
            };
        };
        responses: {
            /** @description Sequence updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_SequenceResponse"];
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
            /** @description Missing `sequences.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Sequence not found (`sequence_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Key already used (`sequence_key_taken`) or the sequence changed since the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid properties or malformed `If-Match` (`validation_error`) */
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
    sequences_enrollments_list: {
        parameters: {
            query?: {
                status?: components["schemas"]["SequenceEnrollmentStatus"];
                page?: number;
                per_page?: number;
            };
            header?: never;
            path: {
                /** @description Sequence id */
                sequence_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Enrollments */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_SequenceEnrollmentResponse"];
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
            /** @description Missing `sequences.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Sequence not found (`sequence_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid status or pagination (`validation_error`) */
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
    sequences_enroll: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Sequence id */
                sequence_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["EnrollContactRequest"];
            };
        };
        responses: {
            /** @description Contact enrolled */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_SequenceEnrollmentResponse"];
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
            /** @description Missing `sequences.enroll` or read access to the contact (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown sequence, contact or mailbox (`sequence_not_found`, `contact_not_found`, `mailbox_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The contact is already active or paused in the sequence (`sequence_already_enrolled`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Inactive or empty sequence (`sequence_inactive`, `sequence_empty`), contact without e-mail (`sequence_contact_without_email`), contact that objected (`sequence_contact_objected`) or no mailbox to send from (`no_mailbox_available`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    sequences_metrics: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Sequence id */
                sequence_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Metrics */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_SequenceMetricsResponse"];
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
            /** @description Missing `sequences.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Sequence not found (`sequence_not_found`) */
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
}
