export interface paths {
    "/api/v1/scoring/models": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Lead scoring models of the organization with their criteria, primary model first within each record type. Needs `scoring.manage`. */
        get: operations["scoring_models_list"];
        put?: never;
        /** @description Creates a scoring model (`scoring_model.created`) for a record type (`contact`, `company`, `deal` or a custom object) and queues a full recalculation. Criteria are `profile` (a `filter` in the same format as views, validated against the fields of the type; `points` when the record matches) or `behavior` (a `signal`: `email_opened`, `email_clicked`, `email_replied`, `meeting_booked`, `form_submitted`, `call_connected`; `points` per occurrence, optional `max_points` cap and `decay_days`: an occurrence loses its points linearly until zero after that many days). Signals of people also count for the scoring models of their company. Bands: cold below `warm_from` (default 30), warm below `hot_from` (default 70), hot from there. The first model of a record type becomes its primary model, exposed as the read-only `score` field. Needs `scoring.manage`. */
        post: operations["scoring_models_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/scoring/models/{model_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["scoring_models_get"];
        put?: never;
        post?: never;
        delete: operations["scoring_models_delete"];
        options?: never;
        head?: never;
        /** @description Updates a scoring model (`scoring_model.updated`). `criteria`, when sent, replaces the whole list (criteria keep their id by `key`, so past signals keep counting). Changing criteria, bands or activation queues a full recalculation. Marking it primary unmarks the previous primary model of the record type. The record type and the key never change. Needs `scoring.manage`. */
        patch: operations["scoring_models_update"];
        trace?: never;
    };
    "/api/v1/scoring/models/{model_id}/recalculate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Queues a full recalculation of the model (job `scoring.recalculate`). Scores are also recalculated by the `scoring.events` consumer and by the daily `scoring.decay` job. Needs `scoring.manage`. */
        post: operations["scoring_models_recalculate"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/scoring/records/{record_type}/{record_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Scores of a record in every active scoring model of its type (primary first), with the band and the explanation: the criteria that added or removed points, and how many behavior occurrences counted. Needs read access to the record. */
        get: operations["scoring_records_get"];
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
        RecordScoreResponse: {
            band: components["schemas"]["ScoreBand"];
            calculated_at?: string | null;
            explanation: components["schemas"]["ScoreContributionResponse"][];
            /** Format: int32 */
            hot_from: number;
            is_primary: boolean;
            model_id: string;
            model_key: string;
            model_name: string;
            /**
             * Format: int32
             * @example 42
             */
            score: number;
            /** Format: int32 */
            warm_from: number;
        };
        ScoreContributionResponse: {
            criterion_id: string;
            key: string;
            kind: components["schemas"]["ScoringCriterionKind"];
            label: string;
            /**
             * Format: int32
             * @example 2
             */
            occurrences: number;
            /**
             * Format: int32
             * @example 18
             */
            points: number;
        };
        /** @enum {string} */
        ScoringCriterionKind: "profile" | "behavior";
        /** @enum {string} */
        ScoreBand: "cold" | "warm" | "hot";
        ScoringCriterionRequest: {
            /** Format: int64 */
            decay_days?: number | null;
            filter?: Record<string, never> | null;
            /** @example replied_email */
            key?: string | null;
            kind: components["schemas"]["ScoringCriterionKind"];
            label: string;
            /** Format: int64 */
            max_points?: number | null;
            /** Format: int64 */
            points: number;
            signal?: null | components["schemas"]["ScoringSignal"];
        };
        /** @enum {string} */
        ScoringSignal: "email_opened" | "email_clicked" | "email_replied" | "meeting_booked" | "form_submitted" | "call_connected";
        ScoringModelResponse: {
            /** @example 2026-10-26T13:45:00Z */
            created_at: string;
            created_by?: string | null;
            criteria: components["schemas"]["ScoringCriterionResponse"][];
            description?: string | null;
            /**
             * Format: int32
             * @example 70
             */
            hot_from: number;
            id: string;
            is_active: boolean;
            is_primary: boolean;
            /** @example lead_score */
            key: string;
            /** @example Lead scoring */
            name: string;
            recalculated_at?: string | null;
            /** @example people.person */
            record_type: string;
            /** @example 2026-10-26T13:45:00Z */
            updated_at: string;
            updated_by?: string | null;
            /**
             * Format: int32
             * @example 30
             */
            warm_from: number;
        };
        ScoringCriterionResponse: {
            /**
             * Format: int32
             * @example 30
             */
            decay_days?: number | null;
            filter?: Record<string, never> | null;
            id: string;
            /** @example replied_email */
            key: string;
            kind: components["schemas"]["ScoringCriterionKind"];
            /** @example Replied to email */
            label: string;
            /**
             * Format: int32
             * @example 30
             */
            max_points?: number | null;
            /**
             * Format: int32
             * @example 10
             */
            points: number;
            /** Format: int32 */
            position: number;
            signal?: null | components["schemas"]["ScoringSignal"];
        };
        Data_Vec_RecordScoreResponse: {
            data: {
                band: components["schemas"]["ScoreBand"];
                calculated_at?: string | null;
                explanation: components["schemas"]["ScoreContributionResponse"][];
                /** Format: int32 */
                hot_from: number;
                is_primary: boolean;
                model_id: string;
                model_key: string;
                model_name: string;
                /**
                 * Format: int32
                 * @example 42
                 */
                score: number;
                /** Format: int32 */
                warm_from: number;
            }[];
        };
        UpdateScoringModelRequest: {
            criteria?: components["schemas"]["ScoringCriterionRequest"][] | null;
            description?: string | null;
            /** Format: int64 */
            hot_from?: number | null;
            is_active?: boolean | null;
            is_primary?: boolean | null;
            name?: string | null;
            /** Format: int64 */
            warm_from?: number | null;
        };
        Data_ScoringModelResponse: {
            data: {
                /** @example 2026-10-26T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                criteria: components["schemas"]["ScoringCriterionResponse"][];
                description?: string | null;
                /**
                 * Format: int32
                 * @example 70
                 */
                hot_from: number;
                id: string;
                is_active: boolean;
                is_primary: boolean;
                /** @example lead_score */
                key: string;
                /** @example Lead scoring */
                name: string;
                recalculated_at?: string | null;
                /** @example people.person */
                record_type: string;
                /** @example 2026-10-26T13:45:00Z */
                updated_at: string;
                updated_by?: string | null;
                /**
                 * Format: int32
                 * @example 30
                 */
                warm_from: number;
            };
        };
        CreateScoringModelRequest: {
            criteria?: components["schemas"]["ScoringCriterionRequest"][];
            description?: string | null;
            /** Format: int64 */
            hot_from?: number | null;
            is_active?: boolean | null;
            is_primary?: boolean | null;
            /** @example lead_score */
            key?: string | null;
            name: string;
            /** @example people.person */
            record_type: string;
            /** Format: int64 */
            warm_from?: number | null;
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
        Data_Vec_ScoringModelResponse: {
            data: {
                /** @example 2026-10-26T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                criteria: components["schemas"]["ScoringCriterionResponse"][];
                description?: string | null;
                /**
                 * Format: int32
                 * @example 70
                 */
                hot_from: number;
                id: string;
                is_active: boolean;
                is_primary: boolean;
                /** @example lead_score */
                key: string;
                /** @example Lead scoring */
                name: string;
                recalculated_at?: string | null;
                /** @example people.person */
                record_type: string;
                /** @example 2026-10-26T13:45:00Z */
                updated_at: string;
                updated_by?: string | null;
                /**
                 * Format: int32
                 * @example 30
                 */
                warm_from: number;
            }[];
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
    scoring_models_list: {
        parameters: {
            query?: {
                record_type?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Scoring models */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_ScoringModelResponse"];
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
            /** @description Missing `scoring.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
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
    scoring_models_create: {
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
                "application/json": components["schemas"]["CreateScoringModelRequest"];
            };
        };
        responses: {
            /** @description Scoring model created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ScoringModelResponse"];
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
            /** @description Missing `scoring.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown record type (`record_type_not_found`) */
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
            /** @description Invalid name, key, bands or criteria (`validation_error` with `details.fields`; filter problems under `criteria.<n>.filter`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    scoring_models_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Scoring model id */
                model_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Scoring model */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ScoringModelResponse"];
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
            /** @description Missing `scoring.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Scoring model not found (`scoring_model_not_found`) */
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
    scoring_models_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Scoring model id */
                model_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Scoring model and its scores removed (`scoring_model.deleted`) */
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
            /** @description Missing `scoring.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Scoring model not found (`scoring_model_not_found`) */
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
    scoring_models_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the model as read (ISO-8601) */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Scoring model id */
                model_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateScoringModelRequest"];
            };
        };
        responses: {
            /** @description Scoring model updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ScoringModelResponse"];
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
            /** @description Missing `scoring.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Scoring model not found (`scoring_model_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The model changed since the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name, key, bands or criteria (`validation_error` with `details.fields`; filter problems under `criteria.<n>.filter`) */
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
    scoring_models_recalculate: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Scoring model id */
                model_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Recalculation queued */
            202: {
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
            /** @description Missing `scoring.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Scoring model not found (`scoring_model_not_found`) */
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
    scoring_records_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Record type, e.g. `contact` */
                record_type: string;
                /** @description Record id */
                record_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Scores of the record */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_RecordScoreResponse"];
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
            /** @description Missing read permission on the record type (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown record type (`record_type_not_found`) or record not visible (`record_not_found`) */
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
