# TypeScript SDK and examples

Aphel offers a TypeScript SDK generated from the OpenAPI document, plus cURL and PHP examples. The source of truth for the contract is still `/api/openapi.json` (versioned copy in `back/openapi.json`); conventions in [style-guide.md](style-guide.md).

## TypeScript SDK (`@aphel-crm/sdk`)

Monorepo package in [`sdk/typescript`](../../sdk/typescript) (ESM, no runtime dependencies, uses the global `fetch`).

| Piece | What it is |
| --- | --- |
| `src/schema.ts` | Types generated with `openapi-typescript` (`npm run generate`). Not kept in git: it is generated from `back/openapi.json` during the build and in CI |
| `AphelClient` | `get`/`post`/`put`/`patch`/`delete` typed by the OpenAPI path (`'/api/v1/contacts/{contact_id}'`), `send` (response with status, headers, rate limit and `replayed`), `paginate`/`listAll` |
| `AphelApiError` | Error built from the `{ "error": { "code", "message", "details" } }` envelope, with `status`, `code`, `details`, `fieldErrors`, `retryAfter` |
| `buildFilterQuery` | `{ amount: { gte: 1000 }, stage_id: { in: ['a', 'b'] } }` → `filter[amount][gte]=1000&filter[stage_id][in]=a,b` |
| Options | `path`, `query`, `body`, `filter`, `filterTree` (AST as JSON), `include`, `sort`, `ifMatch` (the `updated_at` that was read), `idempotencyKey` (`true` generates a UUID), `headers`, `signal`, `timeoutMs` on the client |

```bash
cd sdk/typescript
npm ci
npm run generate                       # from ../../back/openapi.json
npm run generate -- http://127.0.0.1:3001/api/openapi.json
npm run build                          # dist/
npm pack                               # .tgz package ready to install
```

`make sdk` runs `npm ci`, `generate` and `build`. The package is not published to npm for now; install it from the `.tgz` or as a `file:` dependency in a workspace. Full usage in the [package README](../../sdk/typescript/README.md).

```ts
import { createAphelClient, isAphelApiError, unwrap } from '@aphel-crm/sdk'

const aphel = createAphelClient({ baseUrl: 'https://crm.yourcompany.com', token: process.env.APHEL_TOKEN })
const contact = unwrap(await aphel.post('/api/v1/contacts', { body: { first_name: 'Bruna' }, idempotencyKey: true }))
for await (const deal of aphel.paginate('/api/v1/deals', { filter: { status: 'open' } })) console.log(deal.title)
```

### Versions and compatibility

The SDK follows the `/api/v1` contract ([versioning.md](versioning.md)). Since the types come from OpenAPI, regenerate them when you upgrade Aphel or when using an instance with different modules (the `/api/v1/<key>` routes only appear in the document when the module is compiled in). A change that breaks the SDK build shows up in the **TypeScript SDK** CI job, which generates the types and builds the package on every push.

## Best practices

- Keep the token out of the code (environment variable or vault). It has scopes (`contacts:write`, `deals:read`, ...) and is never worth more than the current permissions of the user who created it.
- Use `Idempotency-Key` on a `POST` that may be repeated (queues, retries).
- Use `If-Match` with the `updated_at` you read so you do not overwrite someone else's changes; handle 409 `version_conflict` by re-reading the record.
- Respect `429 rate_limited` and the `Retry-After` header (`AphelApiError.retryAfter`).
- In webhooks, validate the signature over the raw body, reject timestamps more than 5 minutes off and discard repeats by the event `id`.
