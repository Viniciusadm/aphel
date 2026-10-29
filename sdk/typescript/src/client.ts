import { AphelApiError, parseErrorBody } from './errors.js'
import { buildFilterQuery, serializeFilterTree, type FilterConditions, type FilterNode } from './filter.js'
import type { paths } from './schema.js'

export type HttpMethod = 'get' | 'post' | 'put' | 'patch' | 'delete'

export type ApiPath = keyof paths & string

export type PathsWithMethod<M extends HttpMethod> = {
  [P in ApiPath]: paths[P] extends { [K in M]: infer Op } ? ([Op] extends [never | undefined] ? never : P) : never
}[ApiPath]

export type Operation<P extends ApiPath, M extends HttpMethod> = NonNullable<
  paths[P] extends { [K in M]?: infer Op } ? Op : never
>

type JsonContent<R> = R extends { content: { 'application/json': infer Body } } ? Body : undefined

export type SuccessBody<Op> = Op extends { responses: infer Responses }
  ? 200 extends keyof Responses
    ? JsonContent<Responses[200]>
    : 201 extends keyof Responses
      ? JsonContent<Responses[201]>
      : 202 extends keyof Responses
        ? JsonContent<Responses[202]>
        : undefined
  : unknown

export type RequestBody<Op> = Op extends { requestBody?: infer Body }
  ? JsonContent<NonNullable<Body>>
  : never

export type PathParams<Op> = Op extends { parameters: { path?: infer Params } }
  ? [Params] extends [never | undefined]
    ? Record<string, never>
    : NonNullable<Params>
  : Record<string, never>

export type QueryParams<Op> = Op extends { parameters: { query?: infer Params } }
  ? [Params] extends [never | undefined]
    ? Record<string, never>
    : NonNullable<Params>
  : Record<string, never>

export type QueryValue = string | number | boolean | null | undefined | ReadonlyArray<string | number | boolean>

export interface PageMeta {
  total: number
  page: number
  per_page: number
}

export interface Page<T> {
  data: T[]
  meta: PageMeta
}

export type PageItem<Body> = Body extends { data: Array<infer Item> } ? Item : never

export interface RequestOptions<Op> {
  path?: PathParams<Op>
  query?: Partial<QueryParams<Op>> & Record<string, QueryValue>
  body?: RequestBody<Op>
  filter?: FilterConditions
  filterTree?: FilterNode
  include?: readonly string[]
  sort?: readonly string[]
  ifMatch?: string
  idempotencyKey?: string | true
  headers?: Record<string, string>
  signal?: AbortSignal
}

export interface AphelClientOptions {
  baseUrl: string
  token?: string
  fetch?: typeof globalThis.fetch
  headers?: Record<string, string>
  userAgent?: string
  timeoutMs?: number
}

export interface PaginateOptions {
  perPage?: number
  startPage?: number
  maxPages?: number
}

export interface RateLimit {
  limit: number | null
  remaining: number | null
  resetSeconds: number | null
}

export interface AphelResponse<Body> {
  status: number
  headers: Headers
  body: Body
  rateLimit: RateLimit
  replayed: boolean
}

const TOKEN_PATTERN = /^aphel_[A-Za-z0-9]+_[A-Za-z0-9]+$/

function numberHeader(headers: Headers, name: string): number | null {
  const value = headers.get(name)
  if (value === null || value.trim() === '') return null
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

function randomKey(): string {
  const crypto = globalThis.crypto
  if (crypto && typeof crypto.randomUUID === 'function') return crypto.randomUUID()
  return `${Date.now().toString(16)}-${Math.random().toString(16).slice(2)}`
}

function appendQueryValue(search: URLSearchParams, key: string, value: QueryValue): void {
  if (value === undefined) return
  if (value === null) {
    search.append(key, '')
  } else if (Array.isArray(value)) {
    search.append(key, value.map(String).join(','))
  } else {
    search.append(key, String(value))
  }
}

export function isValidTokenFormat(token: string): boolean {
  return TOKEN_PATTERN.test(token)
}

export function versionOf(record: { updated_at?: string | null }): string | undefined {
  return record.updated_at ?? undefined
}

export class AphelClient {
  private readonly baseUrl: string
  private readonly token: string | undefined
  private readonly fetchImpl: typeof globalThis.fetch
  private readonly defaultHeaders: Record<string, string>
  private readonly timeoutMs: number | undefined

  constructor(options: AphelClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/+$/, '')
    this.token = options.token
    this.fetchImpl = options.fetch ?? globalThis.fetch.bind(globalThis)
    this.defaultHeaders = {
      Accept: 'application/json',
      ...(options.userAgent ? { 'User-Agent': options.userAgent } : {}),
      ...options.headers,
    }
    this.timeoutMs = options.timeoutMs
  }

  url<P extends ApiPath>(path: P, pathParams?: Record<string, string | number>, query?: URLSearchParams): string {
    const resolved = path.replace(/\{([^}]+)\}/g, (_, name: string) => {
      const value = pathParams?.[name]
      if (value === undefined) throw new TypeError(`missing path parameter ${name} for ${path}`)
      return encodeURIComponent(String(value))
    })
    const suffix = query && [...query.keys()].length > 0 ? `?${query.toString()}` : ''
    return `${this.baseUrl}${resolved}${suffix}`
  }

  async send<P extends ApiPath, M extends HttpMethod>(
    method: M,
    path: P,
    options: RequestOptions<Operation<P, M>> = {},
  ): Promise<AphelResponse<SuccessBody<Operation<P, M>>>> {
    const search = new URLSearchParams()
    for (const [key, value] of Object.entries(options.query ?? {})) {
      appendQueryValue(search, key, value as QueryValue)
    }
    if (options.filter) {
      for (const [key, value] of buildFilterQuery(options.filter)) search.append(key, value)
    }
    if (options.filterTree) search.set('filter', serializeFilterTree(options.filterTree))
    if (options.include && options.include.length > 0) search.set('include', options.include.join(','))
    if (options.sort && options.sort.length > 0) search.set('sort', options.sort.join(','))

    const headers: Record<string, string> = { ...this.defaultHeaders, ...options.headers }
    if (this.token) headers.Authorization = `Bearer ${this.token}`
    if (options.ifMatch) headers['If-Match'] = options.ifMatch
    if (options.idempotencyKey) {
      headers['Idempotency-Key'] = options.idempotencyKey === true ? randomKey() : options.idempotencyKey
    }
    let body: string | undefined
    if (options.body !== undefined) {
      headers['Content-Type'] = 'application/json'
      body = JSON.stringify(options.body)
    }

    const signals: AbortSignal[] = []
    if (options.signal) signals.push(options.signal)
    if (this.timeoutMs) signals.push(AbortSignal.timeout(this.timeoutMs))
    const signal = signals.length === 0 ? undefined : signals.length === 1 ? signals[0] : AbortSignal.any(signals)

    let response: Response
    try {
      response = await this.fetchImpl(
        this.url(path, options.path as Record<string, string | number> | undefined, search),
        { method: method.toUpperCase(), headers, body, signal },
      )
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === 'AbortError') throw cause
      throw new AphelApiError(0, {
        code: 'network_error',
        message: cause instanceof Error ? cause.message : String(cause),
      })
    }

    const text = await response.text()
    let payload: unknown = undefined
    if (text.length > 0) {
      const contentType = response.headers.get('Content-Type') ?? ''
      if (contentType.includes('json')) {
        try {
          payload = JSON.parse(text)
        } catch {
          payload = text
        }
      } else {
        payload = text
      }
    }
    if (!response.ok) {
      throw new AphelApiError(response.status, parseErrorBody(payload, response.status, response.statusText), response.headers)
    }
    return {
      status: response.status,
      headers: response.headers,
      body: payload as SuccessBody<Operation<P, M>>,
      rateLimit: {
        limit: numberHeader(response.headers, 'X-RateLimit-Limit'),
        remaining: numberHeader(response.headers, 'X-RateLimit-Remaining'),
        resetSeconds: numberHeader(response.headers, 'X-RateLimit-Reset'),
      },
      replayed: response.headers.get('Idempotent-Replayed') === 'true',
    }
  }

  async request<P extends ApiPath, M extends HttpMethod>(
    method: M,
    path: P,
    options?: RequestOptions<Operation<P, M>>,
  ): Promise<SuccessBody<Operation<P, M>>> {
    return (await this.send(method, path, options)).body
  }

  get<P extends PathsWithMethod<'get'>>(path: P, options?: RequestOptions<Operation<P, 'get'>>) {
    return this.request('get', path, options)
  }

  post<P extends PathsWithMethod<'post'>>(path: P, options?: RequestOptions<Operation<P, 'post'>>) {
    return this.request('post', path, options)
  }

  put<P extends PathsWithMethod<'put'>>(path: P, options?: RequestOptions<Operation<P, 'put'>>) {
    return this.request('put', path, options)
  }

  patch<P extends PathsWithMethod<'patch'>>(path: P, options?: RequestOptions<Operation<P, 'patch'>>) {
    return this.request('patch', path, options)
  }

  delete<P extends PathsWithMethod<'delete'>>(path: P, options?: RequestOptions<Operation<P, 'delete'>>) {
    return this.request('delete', path, options)
  }

  async *paginate<P extends PathsWithMethod<'get'>>(
    path: P,
    options: RequestOptions<Operation<P, 'get'>> = {},
    pagination: PaginateOptions = {},
  ): AsyncGenerator<PageItem<SuccessBody<Operation<P, 'get'>>>, void, undefined> {
    const perPage = pagination.perPage ?? 100
    let page = pagination.startPage ?? 1
    let fetched = 0
    for (;;) {
      const query = { ...options.query, page, per_page: perPage } as unknown as RequestOptions<Operation<P, 'get'>>['query']
      const body = (await this.request('get', path, { ...options, query })) as unknown as Page<
        PageItem<SuccessBody<Operation<P, 'get'>>>
      >
      for (const item of body.data) yield item
      fetched += 1
      const total = body.meta?.total ?? 0
      const done = body.data.length === 0 || page * perPage >= total
      if (done || (pagination.maxPages !== undefined && fetched >= pagination.maxPages)) return
      page += 1
    }
  }

  async listAll<P extends PathsWithMethod<'get'>>(
    path: P,
    options?: RequestOptions<Operation<P, 'get'>>,
    pagination?: PaginateOptions,
  ): Promise<Array<PageItem<SuccessBody<Operation<P, 'get'>>>>> {
    const items: Array<PageItem<SuccessBody<Operation<P, 'get'>>>> = []
    for await (const item of this.paginate(path, options, pagination)) items.push(item)
    return items
  }
}

export function createAphelClient(options: AphelClientOptions): AphelClient {
  return new AphelClient(options)
}

export function unwrap<T>(body: { data: T }): T {
  return body.data
}
