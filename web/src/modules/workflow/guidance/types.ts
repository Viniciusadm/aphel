import type { components } from '@/modules/workflow/api-types'

type Schemas = components['schemas']

export type ChecklistItem = Schemas['ChecklistItemResponse']
export type StageGuidance = Schemas['StageGuidanceResponse']
export type ChecklistItemRequest = Schemas['ChecklistItemRequest']
export type StageGuidanceRequest = Schemas['StageGuidanceRequest']
export type ChecklistUser = Schemas['ChecklistUserResponse']
export type RecordChecklistItem = Schemas['RecordChecklistItemResponse']
export type RecordChecklist = Schemas['RecordChecklistResponse']
