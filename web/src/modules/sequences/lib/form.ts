import type { ParseKeys } from 'i18next'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import {
  DEFAULT_DAILY_LIMIT,
  DEFAULT_WINDOW_END,
  DEFAULT_WINDOW_START,
  MAX_DAILY_LIMIT,
  MAX_STEPS,
  MAX_WAIT_DAYS,
  STEP_KINDS,
} from '../constants'
import type { Sequence, SequenceRequest, SequenceStepKind, SequenceStepRequest } from '../types'

export type SequencesKey = ParseKeys<'sequences'>

export function useSequencesText() {
  const { t } = useTranslation('sequences')
  return (key: string) => t(key as SequencesKey)
}

const TIME_PATTERN = /^([01]\d|2[0-3]):[0-5]\d$/
const TASK_KINDS: SequenceStepKind[] = ['call', 'linkedin', 'task']
const EMAIL_KINDS: SequenceStepKind[] = ['auto_email', 'manual_email']

export function isEmailKind(kind: SequenceStepKind): boolean {
  return EMAIL_KINDS.includes(kind)
}

export function isTaskKind(kind: SequenceStepKind): boolean {
  return TASK_KINDS.includes(kind)
}

const stepSchema = z.object({
  id: z.string(),
  kind: z.enum(STEP_KINDS as [SequenceStepKind, ...SequenceStepKind[]]),
  waitDays: z.number(),
  templateId: z.string(),
  subject: z.string(),
  bodyHtml: z.string(),
  title: z.string(),
})

export const sequenceFormSchema = z
  .object({
    name: z.string().trim().min(1, 'validation.nameRequired').max(200, 'validation.nameLength'),
    description: z.string().max(1000, 'validation.descriptionLength'),
    isActive: z.boolean(),
    windowStart: z.string().regex(TIME_PATTERN, 'validation.time'),
    windowEnd: z.string().regex(TIME_PATTERN, 'validation.time'),
    dailyLimit: z
      .number({ message: 'validation.dailyLimit' })
      .int('validation.dailyLimit')
      .min(1, 'validation.dailyLimit')
      .max(MAX_DAILY_LIMIT, 'validation.dailyLimit'),
    steps: z.array(stepSchema).max(MAX_STEPS, 'validation.tooManySteps'),
  })
  .superRefine((values, context) => {
    if (TIME_PATTERN.test(values.windowStart) && TIME_PATTERN.test(values.windowEnd) && values.windowEnd <= values.windowStart) {
      context.addIssue({ code: 'custom', message: 'validation.window', path: ['windowEnd'] })
    }
    let threaded = false
    values.steps.forEach((step, index) => {
      if (step.kind === 'wait' && (!Number.isInteger(step.waitDays) || step.waitDays < 1 || step.waitDays > MAX_WAIT_DAYS)) {
        context.addIssue({ code: 'custom', message: 'validation.waitDays', path: ['steps', index, 'waitDays'] })
      }
      if (isEmailKind(step.kind) && !step.templateId) {
        if (!step.bodyHtml.trim()) {
          context.addIssue({ code: 'custom', message: 'validation.bodyRequired', path: ['steps', index, 'bodyHtml'] })
        }
        if (!step.subject.trim() && !(step.kind === 'auto_email' && threaded)) {
          context.addIssue({ code: 'custom', message: 'validation.subjectRequired', path: ['steps', index, 'subject'] })
        }
      }
      if (isTaskKind(step.kind) && !step.title.trim()) {
        context.addIssue({ code: 'custom', message: 'validation.titleRequired', path: ['steps', index, 'title'] })
      }
      if (step.kind === 'auto_email') threaded = true
    })
  })

export type SequenceFormValues = z.infer<typeof sequenceFormSchema>
export type StepFormValues = SequenceFormValues['steps'][number]

export function emptyStep(kind: SequenceStepKind = 'auto_email'): StepFormValues {
  return { id: '', kind, waitDays: kind === 'wait' ? 2 : 1, templateId: '', subject: '', bodyHtml: '', title: '' }
}

export function defaultValues(sequence: Sequence | null): SequenceFormValues {
  if (!sequence) {
    return {
      name: '',
      description: '',
      isActive: true,
      windowStart: DEFAULT_WINDOW_START,
      windowEnd: DEFAULT_WINDOW_END,
      dailyLimit: DEFAULT_DAILY_LIMIT,
      steps: [emptyStep('auto_email'), emptyStep('wait'), emptyStep('auto_email')],
    }
  }
  return {
    name: sequence.name,
    description: sequence.description ?? '',
    isActive: sequence.is_active,
    windowStart: sequence.window_start.padStart(5, '0'),
    windowEnd: sequence.window_end === '24:00' ? '23:59' : sequence.window_end.padStart(5, '0'),
    dailyLimit: sequence.daily_limit,
    steps: sequence.steps.map((step) => ({
      id: step.id,
      kind: step.kind,
      waitDays: step.kind === 'wait' ? step.wait_days : 1,
      templateId: step.template_id ?? '',
      subject: step.subject,
      bodyHtml: step.body_html ?? '',
      title: step.title,
    })),
  }
}

function stepRequest(step: StepFormValues): SequenceStepRequest {
  const base: SequenceStepRequest = { kind: step.kind }
  if (step.id) base.id = step.id
  if (step.kind === 'wait') return { ...base, wait_days: step.waitDays }
  if (isEmailKind(step.kind)) {
    return {
      ...base,
      template_id: step.templateId || null,
      subject: step.subject.trim(),
      body_html: step.bodyHtml.trim() || null,
      title: step.kind === 'manual_email' ? step.title.trim() : undefined,
    }
  }
  return { ...base, title: step.title.trim() }
}

export function toRequest(values: SequenceFormValues): SequenceRequest {
  return {
    name: values.name.trim(),
    description: values.description.trim() || null,
    is_active: values.isActive,
    window_start: values.windowStart,
    window_end: values.windowEnd,
    daily_limit: values.dailyLimit,
    steps: values.steps.map(stepRequest),
  }
}
