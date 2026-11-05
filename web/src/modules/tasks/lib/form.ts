import { z } from 'zod'
import type {ActivityType, CreateActivityRequest, RecordTarget, RecurrenceRuleRequest} from '@/sdk'
import type { RecurrenceFormState } from './recurrence'

export type ActivityFormMode = 'task' | 'log'

export const activityFormSchema = z.object({
  title: z.string().trim().min(1).max(255),
  activity_type_id: z.string().min(1),
  priority: z.enum(['low', 'normal', 'high', 'urgent']),
  owner_id: z.string(),
  due_at: z.string().nullable(),
  completed_at: z.string().nullable(),
  description: z.string().max(20_000),
  reminder_minutes_before: z.number().int().min(0).nullable(),
  recurrence: z.custom<RecurrenceFormState>(),
  duration_minutes: z
    .string()
    .refine((value) => value.trim() === '' || (/^\d+$/.test(value.trim()) && Number(value) <= 10_080)),
})

export type ActivityFormValues = z.infer<typeof activityFormSchema>

const DEFAULT_TYPE_KEY: Record<ActivityFormMode, string> = { task: 'task', log: 'call' }

export function defaultActivityType(types: ActivityType[], mode: ActivityFormMode): string {
  return (types.find((type) => type.key === DEFAULT_TYPE_KEY[mode]) ?? types[0])?.id ?? ''
}

export function buildCreateRequest(
  values: ActivityFormValues,
  mode: ActivityFormMode,
  target?: RecordTarget,
  recurrence: RecurrenceRuleRequest | null = null,
): CreateActivityRequest {
  const description = values.description.trim()
  const duration = values.duration_minutes.trim()
  return {
    title: values.title.trim(),
    activity_type_id: values.activity_type_id,
    priority: values.priority,
    owner_id: values.owner_id || undefined,
    description: description || undefined,
    due_at: mode === 'task' ? (values.due_at ?? undefined) : undefined,
    status: mode === 'log' ? 'done' : undefined,
    completed_at: mode === 'log' ? (values.completed_at ?? undefined) : undefined,
    duration_minutes: duration ? Number(duration) : undefined,
    reminder_minutes_before: mode === 'task' ? values.reminder_minutes_before : undefined,
    recurrence: mode === 'task' && recurrence ? recurrence : undefined,
    record_type: target?.recordType,
    record_id: target?.recordId,
  }
}
