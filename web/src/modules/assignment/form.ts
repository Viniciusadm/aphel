import { z } from 'zod'
import type { DistributionFallback, DistributionRule, DistributionRuleRequest, DistributionStrategy } from './api'

export const NO_TEAM = 'none'
export const MAX_WEIGHT = 100
export const MAX_NAME_LENGTH = 140

const memberSchema = z
  .object({
    userId: z.string().min(1, 'validation.memberRequired'),
    weight: z.number().int('validation.weight').min(1, 'validation.weight').max(MAX_WEIGHT, 'validation.weight'),
    away: z.boolean(),
    awayFrom: z.string(),
    awayUntil: z.string(),
  })
  .refine((member) => !member.awayFrom || !member.awayUntil || member.awayUntil > member.awayFrom, {
    message: 'validation.awayPeriod',
    path: ['awayUntil'],
  })

const routeSchema = z
  .object({
    values: z.string().refine((text) => splitValues(text).length > 0, 'validation.routeValues'),
    teamId: z.string(),
    userIds: z.array(z.string()),
  })
  .refine((route) => route.teamId !== NO_TEAM || route.userIds.length > 0, {
    message: 'validation.routeTarget',
    path: ['teamId'],
  })

export const ruleFormSchema = z
  .object({
    name: z.string().trim().min(1, 'validation.nameRequired').max(MAX_NAME_LENGTH, 'validation.nameLength'),
    key: z.string().trim(),
    description: z.string(),
    recordType: z.string().min(1, 'validation.recordTypeRequired'),
    strategy: z.enum(['round_robin', 'weighted', 'least_loaded']),
    isActive: z.boolean(),
    members: z.array(memberSchema),
    routingEnabled: z.boolean(),
    routingField: z.string(),
    fallback: z.enum(['members', 'none']),
    routes: z.array(routeSchema),
  })
  .superRefine((values, context) => {
    if (values.key && !/^[a-z][a-z0-9_]*$/.test(values.key)) {
      context.addIssue({ code: 'custom', message: 'validation.key', path: ['key'] })
    }
    const needsMembers = !values.routingEnabled || values.fallback === 'members'
    if (needsMembers && values.members.length === 0) {
      context.addIssue({ code: 'custom', message: 'validation.membersRequired', path: ['members'] })
    }
    const seen = new Set<string>()
    values.members.forEach((member, index) => {
      if (member.userId && seen.has(member.userId)) {
        context.addIssue({ code: 'custom', message: 'validation.memberRepeated', path: ['members', index, 'userId'] })
      }
      seen.add(member.userId)
    })
    if (values.routingEnabled) {
      if (!values.routingField) {
        context.addIssue({ code: 'custom', message: 'validation.routingField', path: ['routingField'] })
      }
      if (values.routes.length === 0) {
        context.addIssue({ code: 'custom', message: 'validation.routesRequired', path: ['routes'] })
      }
    }
  })

export type RuleFormValues = z.infer<typeof ruleFormSchema>
export type MemberFormValues = RuleFormValues['members'][number]
export type RouteFormValues = RuleFormValues['routes'][number]

export function splitValues(text: string): string[] {
  return text
    .split(/[,;\n]/)
    .map((value) => value.trim())
    .filter((value) => value.length > 0)
}

function pad(value: number): string {
  return String(value).padStart(2, '0')
}

export function toLocalInput(iso: string | null | undefined): string {
  if (!iso) return ''
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return ''
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`
}

export function fromLocalInput(local: string): string | null {
  if (!local) return null
  const date = new Date(local)
  return Number.isNaN(date.getTime()) ? null : date.toISOString()
}

export function emptyMember(): MemberFormValues {
  return { userId: '', weight: 1, away: false, awayFrom: '', awayUntil: '' }
}

export function emptyRoute(): RouteFormValues {
  return { values: '', teamId: NO_TEAM, userIds: [] }
}

export function defaultValues(rule: DistributionRule | null, recordType: string): RuleFormValues {
  return {
    name: rule?.name ?? '',
    key: rule?.key ?? '',
    description: rule?.description ?? '',
    recordType: rule?.record_type ?? recordType,
    strategy: (rule?.strategy ?? 'round_robin') as DistributionStrategy,
    isActive: rule?.is_active ?? true,
    members: (rule?.members ?? []).map((member) => ({
      userId: member.user_id,
      weight: member.weight,
      away: member.away,
      awayFrom: toLocalInput(member.away_from),
      awayUntil: toLocalInput(member.away_until),
    })),
    routingEnabled: Boolean(rule?.routing),
    routingField: rule?.routing?.field ?? '',
    fallback: (rule?.routing?.fallback ?? 'members') as DistributionFallback,
    routes: (rule?.routing?.routes ?? []).map((route) => ({
      values: route.values.join(', '),
      teamId: route.team_id ?? NO_TEAM,
      userIds: route.user_ids,
    })),
  }
}

export function toRequest(values: RuleFormValues, creating: boolean): DistributionRuleRequest {
  return {
    name: values.name.trim(),
    key: creating && values.key ? values.key : undefined,
    description: values.description.trim() ? values.description.trim() : null,
    record_type: values.recordType,
    strategy: values.strategy,
    is_active: values.isActive,
    members: values.members.map((member) => ({
      user_id: member.userId,
      weight: values.strategy === 'weighted' ? member.weight : 1,
      away: member.away,
      away_from: member.away ? fromLocalInput(member.awayFrom) : null,
      away_until: member.away ? fromLocalInput(member.awayUntil) : null,
    })),
    routing: values.routingEnabled
      ? {
          field: values.routingField,
          fallback: values.fallback,
          routes: values.routes.map((route) => ({
            values: splitValues(route.values),
            team_id: route.teamId === NO_TEAM ? null : route.teamId,
            user_ids: route.userIds,
          })),
        }
      : null,
  }
}
