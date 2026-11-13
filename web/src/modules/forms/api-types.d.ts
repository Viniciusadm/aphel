export interface paths {
    "/api/v1/forms/forms": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["forms_list"];
        put?: never;
        post: operations["forms_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/forms/forms/{form_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["forms_get"];
        put: operations["forms_update"];
        post?: never;
        delete: operations["forms_delete"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/forms/forms/{form_id}/rotate-public-id": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["forms_rotate_public_id"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/forms/forms/{form_id}/submissions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["forms_submissions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/forms/options": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["forms_options"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/forms/preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["forms_preview"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/forms/forms/{public_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["forms_public_definition"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/forms/forms/{public_id}/embed.js": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["forms_public_embed"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/forms/forms/{public_id}/logo": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["forms_public_logo"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/forms/forms/{public_id}/page": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["forms_public_page"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/forms/forms/{public_id}/submissions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["forms_public_submit"];
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
        /** @enum {string} */
        LeadFormSubmissionStatus: "accepted" | "spam" | "failed";
        LeadFormSubmission: {
            contact_created: boolean;
            contact_id?: string | null;
            created_at: string;
            data: Record<string, never>;
            deal_id?: string | null;
            error_code?: string | null;
            error_message?: string | null;
            form_id: string;
            id: string;
            ip_address?: string | null;
            origin?: string | null;
            status: components["schemas"]["LeadFormSubmissionStatus"];
        };
        LeadFormOptions: {
            contact_fields: components["schemas"]["LeadFormTargetField"][];
            distribution_rules: components["schemas"]["LeadFormDistributionRuleOption"][];
            tags: components["schemas"]["LeadFormTagOption"][];
            users: components["schemas"]["LeadFormUserOption"][];
            workflow: components["schemas"]["LeadFormPipelineOption"][];
        };
        LeadFormPipelineOption: {
            id: string;
            is_default: boolean;
            name: string;
            stages: components["schemas"]["LeadFormStageOption"][];
        };
        LeadFormStageOption: {
            id: string;
            name: string;
        };
        LeadFormUserOption: {
            display_name: string;
            email: string;
            id: string;
        };
        LeadFormTagOption: {
            color: string;
            id: string;
            name: string;
        };
        LeadFormDistributionRuleOption: {
            id: string;
            name: string;
        };
        LeadFormTargetField: {
            custom: boolean;
            field_type: string;
            key: string;
            label: string;
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
        /** @enum {string} */
        LeadFormFieldKind: "text" | "email" | "phone" | "textarea" | "select" | "checkbox";
        LeadFormField: {
            /** @example email */
            key: string;
            kind: components["schemas"]["LeadFormFieldKind"];
            /** @example Your email */
            label: string;
            options?: components["schemas"]["LeadFormOption"][];
            placeholder?: string | null;
            required?: boolean;
            /** @example email */
            target?: string | null;
        };
        LeadFormOption: {
            /** @example Website */
            label: string;
            /** @example website */
            value: string;
        };
        LeadFormResponse: components["schemas"]["LeadFormDefinition"] & {
            /** Format: int64 */
            accepted_count: number;
            acting_user_id?: string | null;
            created_at: string;
            created_by: string;
            /** @example /api/v1/public/forms/forms/f_3b0d/embed.js */
            embed_path: string;
            /** Format: int64 */
            failed_count: number;
            id: string;
            is_active: boolean;
            last_submission_at?: string | null;
            name: string;
            /** @example /api/v1/public/forms/forms/f_3b0d/page */
            page_path: string;
            /** @example f_3b0d1c5e9a8f4f6e8b7a6c5d4e3f2a1b */
            public_id: string;
            /** Format: int64 */
            spam_count: number;
            updated_at: string;
        };
        LeadFormDefinition: {
            allowed_origins?: string[];
            appearance?: null | components["schemas"]["LeadFormAppearance"];
            deal?: null | components["schemas"]["LeadFormDeal"];
            dedupe_by_email?: boolean;
            distribution_rule_id?: string | null;
            fields?: components["schemas"]["LeadFormField"][];
            owner_id?: string | null;
            redirect_url?: string | null;
            /** @example Website form */
            source?: string | null;
            submit_label?: string | null;
            success_message?: string | null;
            tag_id?: string | null;
        };
        LeadFormDeal: {
            /** @example 1500.00 */
            amount?: string | null;
            pipeline_id: string;
            stage_id: string;
            /** @example {{name}} - website form */
            title_template: string;
        };
        LeadFormAppearance: {
            /** @example #2563eb */
            accent?: string | null;
            button?: null | components["schemas"]["ButtonStyle"];
            button_width?: null | components["schemas"]["ButtonWidth"];
            embed_mode?: null | components["schemas"]["EmbedMode"];
            font?: null | components["schemas"]["Font"];
            intro?: string | null;
            radius?: null | components["schemas"]["Radius"];
            show_logo?: boolean | null;
            show_title?: boolean | null;
            surface?: null | components["schemas"]["Surface"];
            theme?: null | components["schemas"]["Theme"];
            width?: null | components["schemas"]["Width"];
        };
        /** @enum {string} */
        Width: "narrow" | "medium" | "wide";
        /** @enum {string} */
        Theme: "clean" | "soft" | "dark" | "outline";
        /** @enum {string} */
        Surface: "plain" | "card";
        /** @enum {string} */
        Radius: "sharp" | "rounded" | "pill";
        /** @enum {string} */
        Font: "system" | "serif" | "rounded";
        /** @enum {string} */
        EmbedMode: "theme" | "inherit";
        /** @enum {string} */
        ButtonWidth: "auto" | "full";
        /** @enum {string} */
        ButtonStyle: "solid" | "outline";
        Data_LeadFormSubmitResult: {
            data: {
                redirect_url?: string | null;
                status: string;
                success_message?: string | null;
            };
        };
        LeadFormSubmitResult: {
            redirect_url?: string | null;
            status: string;
            success_message?: string | null;
        };
        Data_LeadFormPublicDefinition: {
            data: {
                appearance: components["schemas"]["LeadFormPublicAppearance"];
                fields: components["schemas"]["LeadFormPublicField"][];
                /** @example _aphel_hp */
                honeypot_field: string;
                name: string;
                public_id: string;
                redirect_url?: string | null;
                submit_label?: string | null;
                submit_path: string;
                success_message?: string | null;
            };
        };
        LeadFormPublicField: {
            key: string;
            kind: components["schemas"]["LeadFormFieldKind"];
            label: string;
            max_length: number;
            options: components["schemas"]["LeadFormOption"][];
            placeholder?: string | null;
            required: boolean;
        };
        LeadFormPublicAppearance: {
            attributes: {
                [key: string]: string;
            };
            intro?: string | null;
            logo_url?: string | null;
            title?: string | null;
            vars: {
                [key: string]: string;
            };
        };
        LeadFormPublicDefinition: {
            appearance: components["schemas"]["LeadFormPublicAppearance"];
            fields: components["schemas"]["LeadFormPublicField"][];
            /** @example _aphel_hp */
            honeypot_field: string;
            name: string;
            public_id: string;
            redirect_url?: string | null;
            submit_label?: string | null;
            submit_path: string;
            success_message?: string | null;
        };
        Data_LeadFormPreviewResponse: {
            data: {
                html: string;
            };
        };
        LeadFormPreviewResponse: {
            html: string;
        };
        LeadFormPreviewRequest: components["schemas"]["LeadFormRequest"] & {
            /** @example f_3b0d1c5e9a8f4f6e8b7a6c5d4e3f2a1b */
            public_id?: string | null;
        };
        LeadFormRequest: components["schemas"]["LeadFormDefinition"] & {
            acting_user_id?: string | null;
            is_active?: boolean;
            /** @example Website contact */
            name: string;
        };
        Data_LeadFormOptions: {
            data: {
                contact_fields: components["schemas"]["LeadFormTargetField"][];
                distribution_rules: components["schemas"]["LeadFormDistributionRuleOption"][];
                tags: components["schemas"]["LeadFormTagOption"][];
                users: components["schemas"]["LeadFormUserOption"][];
                workflow: components["schemas"]["LeadFormPipelineOption"][];
            };
        };
        Page_LeadFormSubmission: {
            data: {
                contact_created: boolean;
                contact_id?: string | null;
                created_at: string;
                data: Record<string, never>;
                deal_id?: string | null;
                error_code?: string | null;
                error_message?: string | null;
                form_id: string;
                id: string;
                ip_address?: string | null;
                origin?: string | null;
                status: components["schemas"]["LeadFormSubmissionStatus"];
            }[];
            meta: components["schemas"]["PageMeta"];
        };
        Data_LeadFormResponse: {
            data: components["schemas"]["LeadFormDefinition"] & {
                /** Format: int64 */
                accepted_count: number;
                acting_user_id?: string | null;
                created_at: string;
                created_by: string;
                /** @example /api/v1/public/forms/forms/f_3b0d/embed.js */
                embed_path: string;
                /** Format: int64 */
                failed_count: number;
                id: string;
                is_active: boolean;
                last_submission_at?: string | null;
                name: string;
                /** @example /api/v1/public/forms/forms/f_3b0d/page */
                page_path: string;
                /** @example f_3b0d1c5e9a8f4f6e8b7a6c5d4e3f2a1b */
                public_id: string;
                /** Format: int64 */
                spam_count: number;
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
        Page_LeadFormResponse: {
            data: (components["schemas"]["LeadFormDefinition"] & {
                /** Format: int64 */
                accepted_count: number;
                acting_user_id?: string | null;
                created_at: string;
                created_by: string;
                /** @example /api/v1/public/forms/forms/f_3b0d/embed.js */
                embed_path: string;
                /** Format: int64 */
                failed_count: number;
                id: string;
                is_active: boolean;
                last_submission_at?: string | null;
                name: string;
                /** @example /api/v1/public/forms/forms/f_3b0d/page */
                page_path: string;
                /** @example f_3b0d1c5e9a8f4f6e8b7a6c5d4e3f2a1b */
                public_id: string;
                /** Format: int64 */
                spam_count: number;
                updated_at: string;
            })[];
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
    forms_list: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Lead forms, newest first, with submission counters */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_LeadFormResponse"];
                };
            };
            /** @description Missing `forms.manage` (`permission_denied`) */
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
    forms_create: {
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
                "application/json": components["schemas"]["LeadFormRequest"];
            };
        };
        responses: {
            /** @description Form created with a new opaque public id. Publishes `lead_form.created` */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadFormResponse"];
                };
            };
            /** @description Missing `forms.manage` (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The installation reached 200 forms (`lead_form_limit_reached`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid form (`lead_form_invalid` with `details.fields`) or body (`invalid_json`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    forms_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Form id */
                form_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description The form */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadFormResponse"];
                };
            };
            /** @description Missing `forms.manage` (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Form not found (`lead_form_not_found`) */
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
    forms_update: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Form id */
                form_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LeadFormRequest"];
            };
        };
        responses: {
            /** @description Form replaced (the public id is kept). Publishes `lead_form.updated` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadFormResponse"];
                };
            };
            /** @description Missing `forms.manage` (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Form not found (`lead_form_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid form (`lead_form_invalid` with `details.fields`) or body (`invalid_json`) */
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
    forms_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Form id */
                form_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Form and its submission history deleted (people and deals created by it stay). Publishes `lead_form.deleted` */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Missing `forms.manage` (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Form not found (`lead_form_not_found`) */
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
    forms_rotate_public_id: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Form id */
                form_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description New public id: the previous page, script and submission URLs stop working at once. Publishes `lead_form.updated` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadFormResponse"];
                };
            };
            /** @description Missing `forms.manage` (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Form not found (`lead_form_not_found`) */
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
    forms_submissions: {
        parameters: {
            query?: {
                /** @description accepted, spam or failed */
                status?: string;
                /** @description Page (from 1) */
                page?: number;
                /** @description Items per page (1 to 100, default 25) */
                per_page?: number;
            };
            header?: never;
            path: {
                /** @description Form id */
                form_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Submission history of the form, newest first, including spam and failures */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_LeadFormSubmission"];
                };
            };
            /** @description Missing `forms.manage` (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Form not found (`lead_form_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid status or pagination (`lead_form_invalid_query`) */
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
    forms_options: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description What the form editor can pick: writable contact fields (system and custom), active workflow and stages, contact tags, active users and active lead distribution rules for people. Lists the current user cannot read come back empty */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadFormOptions"];
                };
            };
            /** @description Missing `forms.manage` (`permission_denied`) */
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
    forms_preview: {
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
                "application/json": components["schemas"]["LeadFormPreviewRequest"];
            };
        };
        responses: {
            /** @description HTML of the hosted page exactly as visitors would see it for the draft in the body (not saved, not validated beyond the appearance values). `public_id` of an existing form lets the logo load */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadFormPreviewResponse"];
                };
            };
            /** @description Missing `forms.manage` (`permission_denied`) */
            403: {
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
            /** @description Invalid appearance (`lead_form_invalid` with `details.fields`) or body (`invalid_json`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    forms_public_definition: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Public id of the form */
                public_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Public definition of an active form, used by `embed.js` to render it. No authentication */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadFormPublicDefinition"];
                };
            };
            /** @description The request `Origin` is not in the form allowed origins (`lead_form_origin_not_allowed`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown or inactive form, or the module is disabled (`lead_form_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Too many requests from this address (`rate_limited`) */
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
    forms_public_embed: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Public id of the form */
                public_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Script that renders the form inside `<div data-aphel-form="<public_id>">` and submits it with `fetch` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/javascript": string;
                };
            };
            /** @description Unknown or inactive form, or the module is disabled (`lead_form_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Too many requests from this address (`rate_limited`) */
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
    forms_public_logo: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Public id of the form */
                public_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Logo of the organization, only for an active form; used by the hosted page header. No authentication */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "image/png": number[];
                };
            };
            /** @description Unknown or inactive form, no logo configured, or the module is disabled */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Too many requests from this address */
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
    forms_public_page: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Public id of the form */
                public_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Self-contained HTML page with the form, suitable for an iframe. `Content-Security-Policy: frame-ancestors` follows the form allowed origins */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/html": string;
                };
            };
            /** @description Unknown or inactive form, or the module is disabled */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/html": string;
                };
            };
            /** @description Too many requests from this address */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/html": string;
                };
            };
        };
    };
    forms_public_submit: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Public id of the form */
                public_id: string;
            };
            cookie?: never;
        };
        /** @description Values by form field key (up to 64 KB). The honeypot field `_aphel_hp` must stay empty */
        requestBody: {
            content: {
                "application/json": Record<string, never>;
                "application/x-www-form-urlencoded": Record<string, never>;
            };
        };
        responses: {
            /** @description Submission accepted: creates the contact (and the deal), publishes `lead_form.submitted`. A filled honeypot also answers 200 without creating anything (recorded as spam). An HTML form post (`Accept: text/html`) gets the hosted page with the success message or a 303 to the redirect URL */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadFormSubmitResult"];
                };
            };
            /** @description HTML form post with a redirect URL configured */
            303: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description The request `Origin` is not allowed (`lead_form_origin_not_allowed`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown or inactive form, or the module is disabled (`lead_form_not_found`) */
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
            /** @description Invalid values (`lead_form_invalid_submission` with `details.fields` by field key), malformed body (`lead_form_invalid_payload`) or body above 64 KB (`lead_form_payload_too_large`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Too many submissions from this address or to this form (`rate_limited`) */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The acting user is no longer able to create the records (`lead_form_owner_unavailable`) or the CRM refused the contact (`lead_form_submission_failed`); recorded as a failed submission */
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
