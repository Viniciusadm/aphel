import { appPath } from '@/sdk/app-base'
import { ApiError, NETWORK_ERROR_CODE, UNKNOWN_ERROR_CODE, parseErrorEnvelope } from './errors'

export const API_BASE_URL = '/api'
export const CSRF_HEADER = 'X-Requested-With'
export const CSRF_HEADER_VALUE = 'aphel'

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE'

export const TWO_FACTOR_SETUP_REQUIRED_CODE = 'two_factor_setup_required'
export const TWO_FACTOR_SETUP_PATH = '/settings/security'

type ApiErrorListener = (error: ApiError) => void

const errorListeners = new Set<ApiErrorListener>()

export function onApiError(listener: ApiErrorListener): () => void {
  errorListeners.add(listener)
  return () => {
    errorListeners.delete(listener)
  }
}

function redirectToTwoFactorSetup(error: ApiError): void {
  if (error.status !== 403 || error.code !== TWO_FACTOR_SETUP_REQUIRED_CODE) return
  if (typeof window === 'undefined') return
  const setupPath = appPath(TWO_FACTOR_SETUP_PATH)
  if (window.location.pathname.startsWith(setupPath)) return
  const next = new URL(setupPath, window.location.origin)
  next.searchParams.set('reason', TWO_FACTOR_SETUP_REQUIRED_CODE)
  window.location.assign(next.toString())
}

errorListeners.add(redirectToTwoFactorSetup)

function notifyErrorListeners(error: ApiError): void {
  for (const listener of errorListeners) listener(error)
}

export type QueryValue = string | number | boolean | null | undefined
export type QueryParams = Record<string, QueryValue | QueryValue[]>

export interface RequestOptions {
  method?: HttpMethod
  body?: unknown
  query?: QueryParams
  headers?: HeadersInit
  signal?: AbortSignal
}

function buildUrl(path: string, query?: QueryParams): string {
  const normalizedPath = path.startsWith('/') ? path : `/${path}`
  const url = `${API_BASE_URL}${normalizedPath}`
  if (!query) return url
  const params = new URLSearchParams()
  for (const [key, raw] of Object.entries(query)) {
    const values = Array.isArray(raw) ? raw : [raw]
    for (const value of values) {
      if (value !== null && value !== undefined) params.append(key, String(value))
    }
  }
  const search = params.toString()
  return search ? `${url}?${search}` : url
}

function buildHeaders(options: RequestOptions): Headers {
  const headers = new Headers(options.headers)
  headers.set('Accept', 'application/json')
  headers.set(CSRF_HEADER, CSRF_HEADER_VALUE)
  if (options.body !== undefined && !(options.body instanceof FormData)) {
    headers.set('Content-Type', 'application/json')
  }
  return headers
}

function serializeBody(body: unknown): BodyInit | undefined {
  if (body === undefined) return undefined
  if (body instanceof FormData) return body
  return JSON.stringify(body)
}

async function readPayload(response: Response): Promise<unknown> {
  if (response.status === 204) return undefined
  const text = await response.text()
  if (!text) return undefined
  const contentType = response.headers.get('Content-Type') ?? ''
  if (!contentType.includes('json')) return text
  try {
    return JSON.parse(text)
  } catch {
    return text
  }
}

function unwrapDataEnvelope(payload: unknown): unknown {
  if (typeof payload !== 'object' || payload === null || Array.isArray(payload)) return payload
  const keys = Object.keys(payload)
  if (keys.length === 1 && keys[0] === 'data') return (payload as { data: unknown }).data
  return payload
}

export async function apiRequest<T>(path: string, options: RequestOptions = {}): Promise<T> {
  let response: Response
  try {
    response = await fetch(buildUrl(path, options.query), {
      method: options.method ?? 'GET',
      headers: buildHeaders(options),
      body: serializeBody(options.body),
      credentials: 'include',
      signal: options.signal,
    })
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === 'AbortError') throw cause
    throw new ApiError(0, {
      code: NETWORK_ERROR_CODE,
      message: cause instanceof Error ? cause.message : String(cause),
    })
  }

  const payload = await readPayload(response)

  if (!response.ok) {
    const envelope = parseErrorEnvelope(payload)
    const error = new ApiError(
      response.status,
      envelope ?? { code: UNKNOWN_ERROR_CODE, message: response.statusText || `HTTP ${response.status}` },
    )
    notifyErrorListeners(error)
    throw error
  }

  return unwrapDataEnvelope(payload) as T
}

type BodylessOptions = Omit<RequestOptions, 'method' | 'body'>

export const api = {
  get: <T>(path: string, options?: BodylessOptions) => apiRequest<T>(path, { ...options, method: 'GET' }),
  delete: <T>(path: string, options?: BodylessOptions) =>
    apiRequest<T>(path, { ...options, method: 'DELETE' }),
  post: <T>(path: string, body?: unknown, options?: BodylessOptions) =>
    apiRequest<T>(path, { ...options, method: 'POST', body }),
  put: <T>(path: string, body?: unknown, options?: BodylessOptions) =>
    apiRequest<T>(path, { ...options, method: 'PUT', body }),
  patch: <T>(path: string, body?: unknown, options?: BodylessOptions) =>
    apiRequest<T>(path, { ...options, method: 'PATCH', body }),
}
