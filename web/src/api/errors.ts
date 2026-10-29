export interface ApiErrorBody {
  code: string
  message: string
  details?: unknown
}

export const NETWORK_ERROR_CODE = 'network_error'
export const UNKNOWN_ERROR_CODE = 'unknown_error'

export class ApiError extends Error {
  readonly status: number
  readonly code: string
  readonly details: unknown

  constructor(status: number, body: ApiErrorBody) {
    super(body.message)
    this.name = 'ApiError'
    this.status = status
    this.code = body.code
    this.details = body.details ?? null
  }
}

export function isApiError(error: unknown): error is ApiError {
  return error instanceof ApiError
}

function isErrorBody(value: unknown): value is ApiErrorBody {
  if (typeof value !== 'object' || value === null) return false
  const candidate = value as Record<string, unknown>
  return typeof candidate.code === 'string' && typeof candidate.message === 'string'
}

export function parseErrorEnvelope(payload: unknown): ApiErrorBody | null {
  if (typeof payload !== 'object' || payload === null) return null
  const error = (payload as Record<string, unknown>).error
  return isErrorBody(error) ? error : null
}
