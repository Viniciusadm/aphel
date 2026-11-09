import type { components } from '@/api'

type Schemas = components['schemas']

export type FieldType = Schemas['FieldType']
export type FieldOption = Schemas['FieldOptionResponse']
export type FieldDefinition = Schemas['FieldResponse']
export type FieldSchema = Schemas['FieldSchemaResponse']
export type FilterOperator = Schemas['FilterOperator']
export type OperandKind = Schemas['OperandKind']
export type OperatorDefinition = Schemas['OperatorResponse']
export type LogicalOperator = Schemas['LogicalOperator']
export type FilterCondition = Schemas['FilterCondition']
export type FilterGroup = Schemas['FilterGroup']
export type FilterNode = Schemas['FilterNode']
export type FilterRelated = Schemas['FilterRelated']
export type SortDirection = Schemas['SortDirection']
export type SortSpec = Schemas['SortSpec']
export type SavedView = Schemas['SavedViewResponse']
export type SavedViewsPage = Schemas['Page_SavedViewResponse']
export type CreateViewRequest = Schemas['CreateViewRequest']
export type UpdateViewRequest = Schemas['UpdateViewRequest']
export type ViewType = Schemas['ViewType']
export type ViewVisibility = Schemas['ViewVisibility']
export type UserReference = Schemas['UserReferenceResponse']

export interface FieldConfig {
  min?: number | string | null
  max?: number | string | null
  decimals?: number | null
  max_length?: number | null
  currency?: string | null
  pattern?: string | null
  mask?: string | null
  default_country?: string | null
  target_record_type?: string | null
  default_value?: unknown
  expression?: string | null
  result_type?: string | null
}

export type CustomFieldValues = Record<string, unknown>

export interface RecordItem {
  id: string
  custom_fields?: CustomFieldValues
  [key: string]: unknown
}

export interface PageMeta {
  total: number
  page: number
  per_page: number
}

export interface Page<T> {
  data: T[]
  meta: PageMeta
}

export type ArchivedMode = 'exclude' | 'include' | 'only'

export interface RecordListQuery {
  page: number
  perPage: number
  filter?: FilterNode | null
  sort?: SortSpec[]
  search?: string
  archived?: ArchivedMode
}

export type TableDensity = 'compact' | 'comfortable'

export interface RecordViewConfig {
  filter: FilterNode | null
  sort: SortSpec[]
  columns: string[]
  density: TableDensity
}

export function isFilterGroup(node: FilterNode): node is FilterGroup {
  return 'op' in node && 'conditions' in node
}

export function isFilterCondition(node: FilterNode): node is FilterCondition {
  return 'field' in node && 'operator' in node
}

export function isFilterRelated(node: FilterNode): node is FilterRelated {
  return 'related' in node
}
