import type { components } from '@/api'
import type { FilterNode } from '@/features/records/types'

type Schemas = components['schemas']

export type AutomationStatus = Schemas['AutomationStatus']
export type RunStatus = Schemas['AutomationRunStatus']
export type StepStatus = Schemas['AutomationStepStatus']
export type StepType = Schemas['AutomationStepType']

export const AUTOMATION_STATUSES: AutomationStatus[] = ['draft', 'active', 'paused']
export const RUN_STATUSES: RunStatus[] = ['running', 'waiting', 'succeeded', 'failed', 'skipped', 'cancelled']

export type JsonObject = Record<string, unknown>

export interface TriggerSpec {
  type: string
  [option: string]: unknown
}

export type BranchCaseSpec = Schemas['AutomationBranchCase']

export interface StepSpec {
  id: string
  label?: string | null
  type: StepType
  filter?: FilterNode | null
  next?: string | null
  cases?: BranchCaseSpec[] | null
  default?: string | null
  action?: string | null
  config?: JsonObject | null
  duration?: string | null
  until_field?: string | null
  offset?: string | null
  recheck?: FilterNode | null
  timeout?: string | null
  timeout_next?: string | null
}

export interface DefinitionOptions {
  cancel_waiting_on_pause?: boolean
  cancel_waiting_on_publish?: boolean
}

export interface AutomationDefinition {
  start?: string | null
  steps: StepSpec[]
  options?: DefinitionOptions | null
}

export type AutomationStats = Schemas['AutomationStatsResponse']

export interface Automation {
  id: string
  key: string
  name: string
  description?: string | null
  record_type: string
  status: AutomationStatus
  trigger: TriggerSpec
  definition: AutomationDefinition
  version: number
  created_at: string
  created_by?: string | null
  updated_at: string
  updated_by?: string | null
  stats?: AutomationStats | null
}

export interface AutomationVersion {
  id: string
  version: number
  name: string
  record_type: string
  trigger: TriggerSpec
  definition: AutomationDefinition
  published_at: string
  published_by?: string | null
}

export type AutomationRun = Schemas['AutomationRunResponse']

export type RunStep = Omit<Schemas['RunStepResponse'], 'input' | 'output'> & {
  input?: unknown
  output?: unknown
}

export interface AutomationRunDetails extends AutomationRun {
  steps: RunStep[]
}

export type DryRunRequest = Schemas['DryRunRequest']

export type DryRunResult = Omit<Schemas['DryRunResponse'], 'steps'> & { steps: RunStep[] }

export interface JsonSchema {
  type?: string | string[]
  properties?: Record<string, JsonSchema>
  required?: string[]
  enum?: unknown[]
  const?: unknown
  oneOf?: JsonSchema[]
  anyOf?: JsonSchema[]
  items?: JsonSchema
  format?: string
  description?: string
  default?: unknown
  examples?: unknown[]
  minimum?: number
  maximum?: number
  minLength?: number
  maxLength?: number
  maxItems?: number
  readOnly?: boolean
  writeOnly?: boolean
  pattern?: string
  additionalProperties?: boolean | JsonSchema
  'x-field-types'?: string[]
  'x-reference'?: string
}

export interface TriggerCatalogEntry {
  key: string
  label: string
  description: string
  record_types: string[]
  custom_objects: boolean
  manual: boolean
  options_schema: JsonSchema
}

export interface ActionCatalogEntry {
  key: string
  label: string
  description: string
  record_types: string[]
  custom_objects: boolean
  secret_fields: string[]
  config_schema: JsonSchema
}

export interface AutomationCatalog {
  triggers: TriggerCatalogEntry[]
  actions: ActionCatalogEntry[]
  template_roots: string[]
  trigger_paths: string[]
  limits: {
    max_steps: number
    max_chain_depth: number
    runs_per_record_limit: number
    runs_per_record_window_minutes: number
  }
}

export interface AutomationPayload {
  name?: string
  description?: string | null
  trigger?: TriggerSpec
  definition?: AutomationDefinition
}

export interface CreateAutomationPayload {
  name: string
  key?: string
  description?: string | null
  record_type: string
  trigger: TriggerSpec
  definition?: AutomationDefinition
}

export interface AutomationListFilters {
  status?: AutomationStatus
  record_type?: string
  search?: string
  page: number
}

export interface RunListFilters {
  status?: RunStatus
  record_id?: string
  page: number
}
