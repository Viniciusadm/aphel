import type { components } from '@/api'

type Schemas = components['schemas']

export type ValidationRuleTiming = Schemas['ValidationRuleTiming']

export const VALIDATION_RULE_TIMINGS: ValidationRuleTiming[] = ['always', 'create', 'update']

export type ValidationRule = Schemas['ValidationRuleResponse']
export type CreateValidationRuleRequest = Schemas['CreateValidationRuleRequest']
export type UpdateValidationRuleRequest = Schemas['UpdateValidationRuleRequest']
export type ValidationRulePreviewRequest = Schemas['ValidationRulePreviewRequest']
export type ValidationRulePreview = Schemas['ValidationRulePreviewResponse']
