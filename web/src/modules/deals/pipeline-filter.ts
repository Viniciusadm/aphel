import { type FilterNode, isFilterCondition, isFilterGroup } from '@/sdk'
export const PIPELINE_FIELD = 'pipeline_id'

function isPipelineCondition(node: FilterNode): boolean {
  return isFilterCondition(node) && node.field === PIPELINE_FIELD && node.operator === 'eq'
}

export function pipelineFromFilter(filter: FilterNode | null | undefined): string | null {
  if (!filter) return null
  if (isPipelineCondition(filter)) return isFilterCondition(filter) ? String(filter.value ?? '') || null : null
  if (isFilterGroup(filter) && filter.op === 'and') {
    const condition = filter.conditions.find(isPipelineCondition)
    if (condition && isFilterCondition(condition)) return String(condition.value ?? '') || null
  }
  return null
}

export function withPipelineFilter(
  filter: FilterNode | null | undefined,
  pipelineId: string | null,
): FilterNode | null {
  const condition: FilterNode | null = pipelineId ? { field: PIPELINE_FIELD, operator: 'eq', value: pipelineId } : null
  if (!filter || isPipelineCondition(filter)) return condition
  if (isFilterGroup(filter) && filter.op === 'and') {
    const rest = filter.conditions.filter((node) => !isPipelineCondition(node))
    const conditions = condition ? [...rest, condition] : rest
    if (conditions.length === 0) return null
    if (conditions.length === 1) return conditions[0]
    return { op: 'and', conditions }
  }
  return condition ? { op: 'and', conditions: [filter, condition] } : filter
}
