import type { components } from '@/modules/scoring/api-types'
import { type FilterNode } from '@/sdk'
type Schemas = components['schemas']

export type ScoreBand = Schemas['ScoreBand']

export type ScoringCriterionKind = Schemas['ScoringCriterionKind']

export type ScoringSignal = Schemas['ScoringSignal']

export const SCORE_BANDS: ScoreBand[] = ['cold', 'warm', 'hot']

export const SCORING_SIGNALS: ScoringSignal[] = [
  'email_opened',
  'email_clicked',
  'email_replied',
  'meeting_booked',
  'form_submitted',
  'call_connected',
]

export const SCORING_CORE_RECORD_TYPES = ['people.person', 'organizations.organization', 'deals.deal'] as const

export const SCORING_MANAGE = 'scoring.manage'

export const DEFAULT_WARM_FROM = 30
export const DEFAULT_HOT_FROM = 70

export type ScoringCriterion = Omit<Schemas['ScoringCriterionResponse'], 'filter'> & { filter?: FilterNode | null }
export type ScoringModel = Omit<Schemas['ScoringModelResponse'], 'criteria'> & { criteria: ScoringCriterion[] }
export type ScoringCriterionRequest = Omit<Schemas['ScoringCriterionRequest'], 'filter'> & { filter?: FilterNode | null }
export type CreateScoringModelRequest = Omit<Schemas['CreateScoringModelRequest'], 'criteria'> & {
  criteria: ScoringCriterionRequest[]
}
export type UpdateScoringModelRequest = Omit<Schemas['UpdateScoringModelRequest'], 'criteria'> & {
  criteria?: ScoringCriterionRequest[] | null
}
export type ScoreContribution = Schemas['ScoreContributionResponse']
export type RecordScore = Schemas['RecordScoreResponse']
