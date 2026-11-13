import type { EnrollmentStatus, ExitReason, SequenceStepKind } from './types'

export const SEQUENCES_PATH = '/sequences'
export const SEQUENCES_READ_PERMISSION = 'sequences.read'
export const SEQUENCES_MANAGE_PERMISSION = 'sequences.manage'
export const SEQUENCES_ENROLL_PERMISSION = 'sequences.enroll'
export const SEQUENCES_TAB_KEY = 'sequences'
export const SEQUENCES_PER_PAGE = 25
export const ENROLLMENTS_PER_PAGE = 25
export const MAX_STEPS = 30
export const MAX_WAIT_DAYS = 365
export const MAX_DAILY_LIMIT = 2000
export const DEFAULT_WINDOW_START = '08:00'
export const DEFAULT_WINDOW_END = '18:00'
export const DEFAULT_DAILY_LIMIT = 50

export const STEP_KINDS: SequenceStepKind[] = ['auto_email', 'manual_email', 'call', 'linkedin', 'task', 'wait']
export const ENROLLMENT_STATUSES: EnrollmentStatus[] = ['active', 'paused', 'completed', 'exited']
export const EXIT_REASONS: ExitReason[] = [
  'replied',
  'meeting_booked',
  'deal_won',
  'deal_lost',
  'bounced',
  'objection',
  'contact_deleted',
  'removed',
  'contact_unavailable',
  'enroller_unavailable',
  'send_failed',
  'step_failed',
]

export const SEQUENCE_ERROR_CODES = [
  'sequence_not_found',
  'sequence_key_taken',
  'sequence_inactive',
  'sequence_empty',
  'sequence_already_enrolled',
  'sequence_contact_without_email',
  'sequence_contact_objected',
  'sequence_enrollment_not_found',
  'sequence_enrollment_finished',
  'no_mailbox_available',
  'mailbox_not_found',
  'version_conflict',
] as const

export type SequenceErrorCode = (typeof SEQUENCE_ERROR_CODES)[number]

export function sequencePath(id: string): string {
  return `${SEQUENCES_PATH}/${encodeURIComponent(id)}`
}

export function sequenceEditPath(id: string): string {
  return `${sequencePath(id)}/edit`
}
