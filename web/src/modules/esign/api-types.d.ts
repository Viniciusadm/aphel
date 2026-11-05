export interface paths {
    "/api/v1/esign/requests": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Signature requests of a deal, newest first (up to 100). Requires `esign.view` (or `esign.send`) and `deals.read` on the deal. */
        get: operations["esign_requests_list"];
        put?: never;
        /** @description Queues a PDF attached to the deal for electronic signature and publishes `esign.queued`. The worker reauthorizes the request, sends it to Clicksign and publishes `esign.sent` after confirmation. The initial status is `queued`; `delivery_unknown` means provider confirmation is pending or uncertain and must be checked before submitting another request. `quote_id` (optional) ties the request to a quote version of the deal (usually the PDF generated from it by the proposals module) and goes in the event payload. Requires `esign.send` and access to the deal and the file. No legal validity of its own: the validity is the provider's. */
        post: operations["esign_requests_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/esign/requests/{request_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["esign_requests_get"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/esign/requests/{request_id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Cancels a queued request locally or queues cancellation of a sent request (`cancel_pending`, `esign.cancel_queued`). The worker publishes `esign.cancelled` after Clicksign confirms it. Requires `esign.send`. */
        post: operations["esign_requests_cancel"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/esign/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Provider, environment, whether the access token is configured (never the token itself) and the webhook URL to register in Clicksign. Requires `esign.view` or `esign.send`. */
        get: operations["esign_status"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/esign/webhook": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Receives Clicksign webhooks. No session: the body must be signed with the webhook secret configured in the module (`Content-Hmac: sha256=<hex(hmac_sha256(secret, raw body))>`). Idempotent by body: repeated deliveries answer `duplicate`. Updates the signer and request status and publishes `esign.signed`, `esign.refused`, `esign.cancelled`, `esign.completed` or `esign.expired` on the deal. */
        post: operations["esign_public_webhook"];
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
        ContactResponse: {
            address_city?: string | null;
            address_complement?: string | null;
            address_country?: string | null;
            address_district?: string | null;
            address_number?: string | null;
            address_postal_code?: string | null;
            address_state?: string | null;
            address_street?: string | null;
            /** @example 2026-10-20T13:45:00Z */
            archived_at?: string | null;
            company?: null | components["schemas"]["CompanyReferenceResponse"];
            company_id?: string | null;
            /** @example 2026-10-20T13:45:00Z */
            created_at: string;
            created_by?: string | null;
            custom_fields: Record<string, never>;
            email?: string | null;
            first_name: string;
            id: string;
            /** Format: int64 */
            interaction_count: number;
            job_title?: string | null;
            /** @example 2026-10-20T13:45:00Z */
            last_contacted_at?: string | null;
            last_contacted_by?: string | null;
            /** @example 2026-10-20T13:45:00Z */
            last_email_received_at?: string | null;
            /** @example 2026-10-20T13:45:00Z */
            last_email_sent_at?: string | null;
            last_name?: string | null;
            mobile?: string | null;
            name: string;
            notes?: string | null;
            owner?: null | components["schemas"]["OwnerResponse"];
            owner_id?: string | null;
            phone?: string | null;
            possible_duplicates?: components["schemas"]["DuplicateMatchResponse"][] | null;
            source?: string | null;
            /** @example 2026-10-20T13:45:00Z */
            updated_at: string;
        } & {
            [key: string]: unknown;
        };
        DuplicateMatchResponse: {
            record_id: string;
            /**
             * @example [
             *       "email"
             *     ]
             */
            rules: string[];
            title: string;
        };
        OwnerResponse: {
            display_name: string;
            id: string;
        };
        CompanyReferenceResponse: {
            domain?: string | null;
            id: string;
            name: string;
        };
        DealResponse: {
            /** @example 15000.50 */
            amount?: string | null;
            amount_base?: string | null;
            archived_at?: string | null;
            arr?: string | null;
            closed_at?: string | null;
            company?: null | components["schemas"]["DealNamedResponse"];
            company_id?: string | null;
            contact?: null | components["schemas"]["DealNamedResponse"];
            contact_id?: string | null;
            contract_end?: string | null;
            contract_start?: string | null;
            created_at: string;
            created_by?: string | null;
            /** @example BRL */
            currency: string;
            custom_fields: Record<string, never>;
            /**
             * Format: int64
             * @example 12
             */
            days_in_stage: number;
            exchange_rate?: string | null;
            /** @example 2026-12-31 */
            expected_close_date?: string | null;
            id: string;
            is_rotting: boolean;
            last_activity_at?: string | null;
            lost_reason?: null | components["schemas"]["DealNamedResponse"];
            lost_reason_id?: string | null;
            lost_reason_note?: string | null;
            mrr?: string | null;
            owner?: null | components["schemas"]["DealOwnerResponse"];
            owner_id?: string | null;
            pipeline_id: string;
            /** Format: int32 */
            probability?: number | null;
            renewal_of_deal_id?: string | null;
            rotting_at?: string | null;
            source?: string | null;
            stage: components["schemas"]["DealStageResponse"];
            stage_entered_at: string;
            stage_id: string;
            status: components["schemas"]["DealStatus"];
            tcv?: string | null;
            /** @example Acme onboarding */
            title: string;
            updated_at: string;
        } & {
            [key: string]: unknown;
        };
        /** @enum {string} */
        DealStatus: "open" | "won" | "lost";
        DealStageResponse: {
            id: string;
            kind: components["schemas"]["StageKind"];
            name: string;
            /** Format: int32 */
            probability: number;
            /**
             * Format: int32
             * @example 7
             */
            rotting_days?: number | null;
        };
        /** @enum {string} */
        StageKind: "open" | "won" | "lost";
        DealOwnerResponse: {
            display_name: string;
            id: string;
        };
        DealNamedResponse: {
            id: string;
            name: string;
        };
        Data_EsignWebhookResult: {
            data: {
                /** @example applied */
                status: string;
            };
        };
        EsignWebhookResult: {
            /** @example applied */
            status: string;
        };
        Data_EsignStatus: {
            data: {
                configured: boolean;
                /** @example sandbox */
                environment: string;
                legal_notice: string;
                /** @example clicksign */
                provider: string;
                webhook_url: string;
            };
        };
        EsignStatus: {
            configured: boolean;
            /** @example sandbox */
            environment: string;
            legal_notice: string;
            /** @example clicksign */
            provider: string;
            webhook_url: string;
        };
        Data_EsignRequest: {
            data: {
                completed_at?: string | null;
                created_at: string;
                created_by: string;
                deal_id: string;
                /** @example sandbox */
                environment: string;
                file_id: string;
                file_name: string;
                id: string;
                message?: string | null;
                /** @example clicksign */
                provider: string;
                signed_file_id?: string | null;
                signers: components["schemas"]["EsignSigner"][];
                /** @example sent */
                status: string;
                updated_at: string;
            };
        };
        EsignSigner: {
            contact_id?: string | null;
            email: string;
            id: string;
            name: string;
            refused_at?: string | null;
            signed_at?: string | null;
            /** @example pending */
            status: string;
        };
        EsignRequest: {
            completed_at?: string | null;
            created_at: string;
            created_by: string;
            deal_id: string;
            /** @example sandbox */
            environment: string;
            file_id: string;
            file_name: string;
            id: string;
            message?: string | null;
            /** @example clicksign */
            provider: string;
            signed_file_id?: string | null;
            signers: components["schemas"]["EsignSigner"][];
            /** @example sent */
            status: string;
            updated_at: string;
        };
        CreateEsignRequest: {
            deal_id: string;
            file_id: string;
            message?: string | null;
            quote_id?: string | null;
            signers: components["schemas"]["EsignSignerInput"][];
        };
        EsignSignerInput: {
            contact_id?: string | null;
            email: string;
            name: string;
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
        Data_Vec_EsignRequest: {
            data: {
                completed_at?: string | null;
                created_at: string;
                created_by: string;
                deal_id: string;
                /** @example sandbox */
                environment: string;
                file_id: string;
                file_name: string;
                id: string;
                message?: string | null;
                /** @example clicksign */
                provider: string;
                signed_file_id?: string | null;
                signers: components["schemas"]["EsignSigner"][];
                /** @example sent */
                status: string;
                updated_at: string;
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
    esign_requests_list: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                deal_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Signature requests */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_EsignRequest"];
                };
            };
            /** @description Missing permission (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found or not visible (`deal_not_found`) or module disabled (`module_disabled`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid deal id (`esign_invalid_request`) */
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
    esign_requests_create: {
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
                "application/json": components["schemas"]["CreateEsignRequest"];
            };
        };
        responses: {
            /** @description Request queued */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_EsignRequest"];
                };
            };
            /** @description Missing `esign.send` or no access to the file (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found (`deal_not_found`), file not found or module disabled (`module_disabled`) */
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
            /** @description Invalid request (`esign_invalid_request`), file not attached to the deal (`esign_file_not_attached`), not a PDF (`esign_file_not_pdf`), above 20 MB (`esign_file_too_large`) or refused by the provider (`esign_provider_rejected`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
            /** @description Access token not configured (`esign_not_configured`), refused (`esign_provider_unauthorized`) or provider unavailable (`esign_provider_unavailable`) */
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
    esign_requests_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Signature request id */
                request_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Signature request */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_EsignRequest"];
                };
            };
            /** @description Missing permission (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Request not found (`esign_request_not_found`), deal not visible (`deal_not_found`) or module disabled (`module_disabled`) */
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
    esign_requests_cancel: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Signature request id */
                request_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Request cancelled locally or cancellation queued */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_EsignRequest"];
                };
            };
            /** @description Missing permission (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Request not found (`esign_request_not_found`) or module disabled (`module_disabled`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Request already closed (`esign_request_closed`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Refused by the provider (`esign_provider_rejected`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
            /** @description Provider not configured or unavailable (`esign_not_configured`, `esign_provider_unauthorized`, `esign_provider_unavailable`) */
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
    esign_status: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Module status */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_EsignStatus"];
                };
            };
            /** @description Missing permission (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Module disabled (`module_disabled`) */
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
    esign_public_webhook: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path?: never;
            cookie?: never;
        };
        /** @description Clicksign webhook payload (up to 512 KB) */
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description `applied`, `ignored` (event without effect or unknown envelope) or `duplicate` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_EsignWebhookResult"];
                };
            };
            /** @description Missing or invalid `Content-Hmac` (`invalid_webhook_signature`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Module disabled (`module_disabled`) */
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
            /** @description Body is not JSON (`esign_invalid_webhook_payload`) or above 512 KB (`esign_webhook_payload_too_large`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Too many requests (`rate_limited`) */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Webhook secret not configured (`esign_webhook_not_configured`) or no member able to act for the request (`esign_owner_unavailable`) */
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
