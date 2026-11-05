import type { components } from '@/modules/sequences/api-types'

type Schemas = components['schemas']

export type { RecordsPage as Page, PageMeta } from '@/sdk'

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
