export interface paths {
    "/api/v1/me/followed-records": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Records the caller follows and can still read, most recently followed first, optionally of a single `record_type`. */
        get: operations["followed_records_list"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/records/{record_type}/{record_id}/follow": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description The caller starts following a record they can read. Following twice is a no-op. Following is a personal preference: it publishes no event. */
        post: operations["record_follow"];
        /** @description The caller stops following a record (also when they follow it as owner; they only follow again if they become the owner again). Works even when the record is no longer readable. */
        delete: operations["record_unfollow"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/records/{record_type}/{record_id}/followers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Active members who follow the record and whether the caller follows it. Followers are notified (`followed_record_activity`) of generic record changes and owner-provided related events while the source and target remain readable. The owner follows automatically (`origin: owner`). */
        get: operations["record_followers_list"];
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
        FollowedRecordResponse: {
            followed_at: string;
            /** @example owner */
            origin: string;
            record_id: string;
            /** @example deals.deal */
            record_type: string;
            title?: string | null;
        };
        RecordFollowersResponse: {
            followers: components["schemas"]["RecordFollowerResponse"][];
            following: boolean;
            record_id: string;
            record_type: string;
            total: number;
        };
        RecordFollowerResponse: {
            followed_at: string;
            /** @example manual */
            origin: string;
            user: components["schemas"]["UserReferenceResponse"];
        };
        UserReferenceResponse: {
            display_name: string;
            id: string;
        };
        Data_RecordFollowersResponse: {
            data: {
                followers: components["schemas"]["RecordFollowerResponse"][];
                following: boolean;
                record_id: string;
                record_type: string;
                total: number;
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
        Page_FollowedRecordResponse: {
            data: {
                followed_at: string;
                /** @example owner */
                origin: string;
                record_id: string;
                /** @example deals.deal */
                record_type: string;
                title?: string | null;
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
    followed_records_list: {
        parameters: {
            query?: {
                /** @example deals.deal */
                record_type?: string;
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Followed records */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_FollowedRecordResponse"];
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
            /** @description Unknown record type (`record_type_not_found`) */
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
    record_follow: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Record type */
                record_type: string;
                /** @description Record id */
                record_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Followers of the record after following */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_RecordFollowersResponse"];
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
            /** @description Missing the read permission of the type (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown type or record not visible (`record_type_not_found`, `record_not_found`) */
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
    record_unfollow: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Record type */
                record_type: string;
                /** @description Record id */
                record_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Followers of the record after unfollowing (empty when the record is not readable) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_RecordFollowersResponse"];
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
            /** @description Unknown type or invalid id (`record_type_not_found`, `record_not_found`) */
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
    record_followers_list: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Record type, such as `deal` or `custom.contract` */
                record_type: string;
                /** @description Record id */
                record_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Followers of the record */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_RecordFollowersResponse"];
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
            /** @description Missing the read permission of the type (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown type or record not visible (`record_type_not_found`, `record_not_found`) */
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
