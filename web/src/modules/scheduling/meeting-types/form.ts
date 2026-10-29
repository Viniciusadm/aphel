import { z } from 'zod'
import type {
  MeetingLocationKind,
  MeetingQuestionKind,
  MeetingTypeRequest,
  MeetingTypeResponse,
} from '../types'

export const MAX_QUESTIONS = 10
export const MIN_DURATION = 5
export const MAX_DURATION = 480
export const MIN_SLOT_INTERVAL = 5
export const MAX_SLOT_INTERVAL = 240
export const NO_CONTACT_FIELD = 'none'
export const NO_DISTRIBUTION_RULE = 'none'

const SLUG_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/

function integerIn(min: number, max: number, message: string) {
  return z.number({ error: message }).int(message).min(min, message).max(max, message)
}

const questionSchema = z.object({
  key: z.string(),
  label: z.string().trim().min(1, 'validation.questionLabel').max(200, 'validation.questionLabel'),
  kind: z.enum(['text', 'long_text']),
  required: z.boolean(),
  contactField: z.string(),
})

export const meetingTypeFormSchema = z
  .object({
    name: z.string().trim().min(1, 'validation.nameRequired').max(140, 'validation.nameLength'),
    slug: z.string().trim(),
    description: z.string(),
    durationMinutes: integerIn(MIN_DURATION, MAX_DURATION, 'validation.duration'),
    slotIntervalMinutes: integerIn(MIN_SLOT_INTERVAL, MAX_SLOT_INTERVAL, 'validation.slotInterval'),
    locationKind: z.enum(['none', 'video', 'address', 'phone']),
    locationValue: z.string(),
    spamProtection: z.boolean(),
    isActive: z.boolean(),
    distributionRuleId: z.string(),
    questions: z.array(questionSchema).max(MAX_QUESTIONS, 'validation.tooManyQuestions'),
  })
  .superRefine((values, context) => {
    if (values.slug && !SLUG_PATTERN.test(values.slug)) {
      context.addIssue({ code: 'custom', message: 'validation.slug', path: ['slug'] })
    }
    if (values.locationKind !== 'none' && !values.locationValue.trim()) {
      context.addIssue({ code: 'custom', message: 'validation.locationValue', path: ['locationValue'] })
    }
  })

export type MeetingTypeFormValues = z.infer<typeof meetingTypeFormSchema>
export type QuestionFormValues = MeetingTypeFormValues['questions'][number]

export function emptyQuestion(): QuestionFormValues {
  return { key: '', label: '', kind: 'text', required: false, contactField: NO_CONTACT_FIELD }
}

export function defaultValues(meetingType: MeetingTypeResponse | null): MeetingTypeFormValues {
  return {
    name: meetingType?.name ?? '',
    slug: meetingType?.slug ?? '',
    description: meetingType?.description ?? '',
    durationMinutes: meetingType?.duration_minutes ?? 30,
    slotIntervalMinutes: meetingType?.slot_interval_minutes ?? 30,
    locationKind: (meetingType?.location_kind ?? 'video') as MeetingLocationKind,
    locationValue: meetingType?.location_value ?? '',
    spamProtection: meetingType?.spam_protection ?? true,
    isActive: meetingType?.is_active ?? true,
    distributionRuleId: meetingType?.distribution_rule?.id ?? NO_DISTRIBUTION_RULE,
    questions: (meetingType?.questions ?? []).map((question) => ({
      key: question.key,
      label: question.label,
      kind: question.kind as MeetingQuestionKind,
      required: question.required,
      contactField: question.contact_field ?? NO_CONTACT_FIELD,
    })),
  }
}

export function toRequest(values: MeetingTypeFormValues, teamLinksAvailable: boolean): MeetingTypeRequest {
  const description = values.description.trim()
  const request: MeetingTypeRequest = {
    name: values.name.trim(),
    slug: values.slug ? values.slug : undefined,
    description: description ? description : null,
    duration_minutes: values.durationMinutes,
    slot_interval_minutes: values.slotIntervalMinutes,
    location_kind: values.locationKind,
    location_value: values.locationKind === 'none' ? null : values.locationValue.trim(),
    spam_protection: values.spamProtection,
    is_active: values.isActive,
    questions: values.questions.map((question) => ({
      key: question.key || undefined,
      label: question.label.trim(),
      kind: question.kind,
      required: question.required,
      contact_field: question.contactField === NO_CONTACT_FIELD ? null : question.contactField,
    })),
  }
  if (teamLinksAvailable) {
    request.distribution_rule_id =
      values.distributionRuleId === NO_DISTRIBUTION_RULE ? null : values.distributionRuleId
  }
  return request
}

export const serverFieldPaths = {
  name: 'name',
  slug: 'slug',
  description: 'description',
  duration_minutes: 'durationMinutes',
  slot_interval_minutes: 'slotIntervalMinutes',
  location_kind: 'locationKind',
  location_value: 'locationValue',
  distribution_rule_id: 'distributionRuleId',
  questions: 'questions',
} as const satisfies Record<string, keyof MeetingTypeFormValues>
