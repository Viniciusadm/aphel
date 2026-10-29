import { isApiError } from '@/api'
import { APPROVAL_ERROR_CODES, APPROVAL_PENDING, APPROVAL_REQUIRED, type ApprovalErrorCode } from './constants'
import type { ApprovalMoment, ApprovalPendingDetails, ApprovalRequiredDetails } from './types'

export type ApprovalErrorKey = `errors.${ApprovalErrorCode}` | 'errors.generic'

export function approvalErrorKey(error: unknown): ApprovalErrorKey {
  if (isApiError(error) && (APPROVAL_ERROR_CODES as readonly string[]).includes(error.code)) {
    return `errors.${error.code as ApprovalErrorCode}`
  }
  return 'errors.generic'
}

function detailsOf(error: unknown): Record<string, unknown> {
  if (!isApiError(error) || typeof error.details !== 'object' || error.details === null) return {}
  return error.details as Record<string, unknown>
}

function text(value: unknown): string | null {
  return typeof value === 'string' && value !== '' ? value : null
}

export function isApprovalGateError(error: unknown): boolean {
  return isApiError(error) && (error.code === APPROVAL_REQUIRED || error.code === APPROVAL_PENDING)
}

export function approvalRequiredDetails(error: unknown): ApprovalRequiredDetails | null {
  if (!isApiError(error) || error.code !== APPROVAL_REQUIRED) return null
  const details = detailsOf(error)
  const recordType = text(details.record_type)
  const recordId = text(details.record_id)
  const moment = text(details.moment) as ApprovalMoment | null
  if (!recordType || !recordId || !moment) return null
  const policies = Array.isArray(details.policies) ? (details.policies as ApprovalRequiredDetails['policies']) : []
  return {
    record_type: recordType,
    record_id: recordId,
    moment,
    target_id: text(details.target_id),
    policies,
  }
}

export function approvalPendingDetails(error: unknown): ApprovalPendingDetails | null {
  if (!isApiError(error) || error.code !== APPROVAL_PENDING) return null
  const details = detailsOf(error)
  return {
    request_id: text(details.request_id) ?? '',
    record_type: text(details.record_type) ?? undefined,
    record_id: text(details.record_id) ?? undefined,
    moment: (text(details.moment) as ApprovalMoment | null) ?? undefined,
    target_id: text(details.target_id),
    fields: Array.isArray(details.fields)
      ? details.fields.filter((field): field is string => typeof field === 'string')
      : undefined,
  }
}
