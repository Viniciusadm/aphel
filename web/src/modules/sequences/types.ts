import type { components } from '@/api'

type Schemas = components['schemas']

export type { Page, PageMeta } from '@/features/records/types'

export type SequenceStepKind = Schemas['SequenceStepKind']
export type EnrollmentStatus = Schemas['SequenceEnrollmentStatus']
export type EnrollmentSource = Schemas['SequenceEnrollmentSource']
export type ExitReason = Schemas['SequenceExitReason']
export type SequenceStep = Schemas['SequenceStepResponse']
export type EnrollmentCounts = Schemas['SequenceEnrollmentCountsResponse']
export type Sequence = Schemas['SequenceResponse']
export type SequenceStepRequest = Schemas['SequenceStepRequest']
export type SequenceRequest = Schemas['SequenceRequest']
export type StepMetrics = Schemas['SequenceStepMetricsResponse']
export type SequenceMetrics = Schemas['SequenceMetricsResponse']
export type Enrollment = Schemas['SequenceEnrollmentResponse']
export type EnrollRequest = Schemas['EnrollContactRequest']
