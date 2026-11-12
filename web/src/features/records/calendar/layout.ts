import { activeFields } from '../fields'
import type { FieldDefinition, FieldSchema, SavedView } from '../types'

export type ListLayout = 'table' | 'calendar' | 'list'

export const CALENDAR_VIEW_TYPE = 'calendar'
export const CALENDAR_FIELD_SETTING = 'calendar_field'
const PREFERRED_CALENDAR_FIELDS = ['due_at', 'expected_close_date']

export function isCalendarField(field: FieldDefinition): boolean {
  return (
    (field.field_type === 'date' || field.field_type === 'datetime') &&
    field.access !== 'hidden' &&
    field.is_filterable
  )
}

export function calendarFields(schema: FieldSchema): FieldDefinition[] {
  return activeFields(schema).filter(isCalendarField)
}

export function pickCalendarField(fields: FieldDefinition[], requested?: string | null): FieldDefinition | null {
  if (requested) {
    const match = fields.find((field) => field.key === requested)
    if (match) return match
  }
  for (const key of PREFERRED_CALENDAR_FIELDS) {
    const match = fields.find((field) => field.key === key)
    if (match) return match
  }
  return fields[0] ?? null
}

export interface ViewCalendarConfig {
  layout: ListLayout
  field: string | null
}

export function viewCalendarConfig(view: SavedView | undefined): ViewCalendarConfig {
  const settings = (view?.settings ?? {}) as Record<string, unknown>
  const field = settings[CALENDAR_FIELD_SETTING]
  return {
    layout: view?.view_type === CALENDAR_VIEW_TYPE ? 'calendar' : 'table',
    field: typeof field === 'string' && field ? field : null,
  }
}

export function parseLayout(raw: string | null): ListLayout | undefined {
  return raw === 'calendar' || raw === 'table' || raw === 'list' ? raw : undefined
}
