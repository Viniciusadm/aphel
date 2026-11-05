import { type FilterNode, isFilterCondition, isFilterGroup } from '@/sdk'
import { IS_ROTTING_FIELD } from '@/sdk'

const ROTTING_CONDITION: FilterNode = { field: IS_ROTTING_FIELD, operator: 'eq', value: true }

function isRottingCondition(node: FilterNode): boolean {
  return isFilterCondition(node) && node.field === IS_ROTTING_FIELD && node.operator === 'eq' && node.value === true
}

export function hasRottingFilter(filter: FilterNode | null | undefined): boolean {
  if (!filter) return false
  if (isRottingCondition(filter)) return true
  return isFilterGroup(filter) && filter.op === 'and' && filter.conditions.some(isRottingCondition)
}

export function withRottingFilter(filter: FilterNode | null | undefined, enabled: boolean): FilterNode | null {
  if (!filter || isRottingCondition(filter)) return enabled ? ROTTING_CONDITION : null
  if (isFilterGroup(filter) && filter.op === 'and') {
    const rest = filter.conditions.filter((node) => !isRottingCondition(node))
    const conditions = enabled ? [...rest, ROTTING_CONDITION] : rest
    if (conditions.length === 0) return null
    if (conditions.length === 1) return conditions[0]
    return { op: 'and', conditions }
  }
  return enabled ? { op: 'and', conditions: [filter, ROTTING_CONDITION] } : filter
}
