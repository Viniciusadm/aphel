import type { FieldDefinition, FilterNode } from '../types'
import { zonedDayStart } from './dates'

export function rangeFilter(
  field: FieldDefinition,
  from: string,
  to: string,
  timeZone: string,
  filter: FilterNode | null,
): FilterNode {
  const value = (key: string) => (field.field_type === 'date' ? key : zonedDayStart(key, timeZone))
  const conditions: FilterNode[] = [
    { field: field.key, operator: 'gte', value: value(from) },
    { field: field.key, operator: 'lt', value: value(to) },
  ]
  return { op: 'and', conditions: filter ? [filter, ...conditions] : conditions }
}
