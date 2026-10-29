import { useMemo } from 'react'
import { useSession } from '@/app/session/session-context'
import { getFieldValue, type RecordValues } from '../fields'
import { useRecordFormatting } from '../formatting'
import type { FieldDefinition, RecordItem } from '../types'
import { ruleState, type ConditionContext, type RuleState } from './lib/conditions'
import { buildConditionContext, type ResolvedLayout, resolvedFields } from './lib/layout'

export function useConditionContext(fields: FieldDefinition[]): ConditionContext {
  const { me } = useSession()
  const { timezone } = useRecordFormatting()
  return useMemo(
    () => buildConditionContext(fields, { timeZone: timezone, userId: me.user.id }),
    [fields, timezone, me.user.id],
  )
}

export function recordRuleValues(fields: FieldDefinition[], record?: RecordItem, defaults?: RecordValues): RecordValues {
  return Object.fromEntries(
    fields
      .filter((field) => field.is_active)
      .map((field) => [field.key, record ? getFieldValue(record, field) : (defaults?.[field.key] ?? null)]),
  )
}

export function layoutRuleStates(
  layout: ResolvedLayout | null,
  values: RecordValues,
  context: ConditionContext,
): Record<string, RuleState> {
  if (!layout) return {}
  return Object.fromEntries(resolvedFields(layout).map(({ field, rule }) => [field.key, ruleState(rule, values, context)]))
}
