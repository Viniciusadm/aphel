export interface AphelErrorBody {
  code: string
  message: string
  details?: unknown
}

export class AphelApiError extends Error {
  readonly status: number
  readonly code: string
  readonly details: unknown
  readonly retryAfter: number | null
  readonly requestId: string | null

  constructor(status: number, body: AphelErrorBody, headers?: Headers) {
    super(body.message)
    this.name = 'AphelApiError'
    this.status = status
    this.code = body.code
    this.details = body.details ?? null
    const retryAfter = headers?.get('Retry-After')
    this.retryAfter = retryAfter && Number.isFinite(Number(retryAfter)) ? Number(retryAfter) : null
    this.requestId = headers?.get('X-Request-Id') ?? null
  }

  get fieldErrors(): Array<{ field: string; message: string }> {
    const details = this.details as { fields?: unknown } | null
    if (!details || !Array.isArray(details.fields)) return []
    return details.fields.filter(
      (item): item is { field: string; message: string } =>
        typeof item === 'object' && item !== null && 'field' in item && 'message' in item,
    )
  }
}

export function isAphelApiError(error: unknown, code?: string): error is AphelApiError {
  return error instanceof AphelApiError && (code === undefined || error.code === code)
}

export function parseErrorBody(payload: unknown, status: number, statusText: string): AphelErrorBody {
  if (typeof payload === 'object' && payload !== null && 'error' in payload) {
    const error = (payload as { error: unknown }).error
    if (typeof error === 'object' && error !== null && 'code' in error) {
      const { code, message, details } = error as { code: unknown; message?: unknown; details?: unknown }
      return {
        code: String(code),
        message: typeof message === 'string' ? message : statusText,
        details,
      }
    }
  }
  return { code: status === 0 ? 'network_error' : 'unknown_error', message: statusText || `HTTP ${status}` }
}
