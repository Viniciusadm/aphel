import type { TFunction } from 'i18next'
import type {ActivityRecurrence, RecurrenceFrequency, RecurrenceRuleRequest, RecurrenceWeekday} from '@/sdk'

export const RECURRENCE_FREQUENCIES: RecurrenceFrequency[] = ['daily', 'weekly', 'monthly', 'yearly']
export const RECURRENCE_WEEKDAYS: RecurrenceWeekday[] = ['mo', 'tu', 'we', 'th', 'fr', 'sa', 'su']
export const REMINDER_OPTIONS: (number | null)[] = [null, 0, 5, 15, 30, 60, 1440]
export const DEFAULT_REMINDER_MINUTES = 60
export const MAX_RECURRENCE_INTERVAL = 365
export const MAX_RECURRENCE_COUNT = 1000

export type RecurrenceEnd = 'never' | 'until' | 'count'

export interface RecurrenceFormState {
  frequency: RecurrenceFrequency | 'none'
  interval: string
  weekdays: RecurrenceWeekday[]
  monthDay: string
  end: RecurrenceEnd
  until: string
  count: string
}

export type RecurrenceErrorKey =
  | 'recurrence.errors.dueRequired'
  | 'recurrence.errors.interval'
  | 'recurrence.errors.monthDay'
  | 'recurrence.errors.until'
  | 'recurrence.errors.count'

export const EMPTY_RECURRENCE: RecurrenceFormState = {
  frequency: 'none',
  interval: '1',
  weekdays: [],
  monthDay: '',
  end: 'never',
  until: '',
  count: '',
}

export function recurrenceFromActivity(recurrence: ActivityRecurrence | null | undefined): RecurrenceFormState {
  if (!recurrence || recurrence.ended) return EMPTY_RECURRENCE
  return {
    frequency: recurrence.frequency,
    interval: String(recurrence.interval),
    weekdays: recurrence.by_weekday,
    monthDay: recurrence.month_day ? String(recurrence.month_day) : '',
    end: recurrence.until ? 'until' : recurrence.count ? 'count' : 'never',
    until: recurrence.until ?? '',
    count: recurrence.count ? String(recurrence.count) : '',
  }
}

function integerIn(value: string, min: number, max: number): number | null {
  const trimmed = value.trim()
  if (!/^\d+$/.test(trimmed)) return null
  const number = Number(trimmed)
  return number >= min && number <= max ? number : null
}

export function recurrenceError(state: RecurrenceFormState, dueAt: string | null): RecurrenceErrorKey | null {
  if (state.frequency === 'none') return null
  if (!dueAt) return 'recurrence.errors.dueRequired'
  if (integerIn(state.interval, 1, MAX_RECURRENCE_INTERVAL) === null) return 'recurrence.errors.interval'
  if (state.frequency === 'monthly' && state.monthDay.trim() !== '' && integerIn(state.monthDay, 1, 31) === null) {
    return 'recurrence.errors.monthDay'
  }
  if (state.end === 'until' && !/^\d{4}-\d{2}-\d{2}$/.test(state.until)) return 'recurrence.errors.until'
  if (state.end === 'count' && integerIn(state.count, 1, MAX_RECURRENCE_COUNT) === null) return 'recurrence.errors.count'
  return null
}

export function buildRecurrenceRule(state: RecurrenceFormState): RecurrenceRuleRequest | null {
  if (state.frequency === 'none') return null
  const monthDay = state.frequency === 'monthly' ? integerIn(state.monthDay, 1, 31) : null
  return {
    frequency: state.frequency,
    interval: integerIn(state.interval, 1, MAX_RECURRENCE_INTERVAL) ?? 1,
    by_weekday: state.frequency === 'weekly' && state.weekdays.length > 0 ? sortWeekdays(state.weekdays) : undefined,
    month_day: monthDay ?? undefined,
    until: state.end === 'until' ? state.until : undefined,
    count: state.end === 'count' ? (integerIn(state.count, 1, MAX_RECURRENCE_COUNT) ?? undefined) : undefined,
  }
}

export function sortWeekdays(weekdays: RecurrenceWeekday[]): RecurrenceWeekday[] {
  return RECURRENCE_WEEKDAYS.filter((weekday) => weekdays.includes(weekday))
}

export function toggleWeekday(weekdays: RecurrenceWeekday[], weekday: RecurrenceWeekday): RecurrenceWeekday[] {
  return weekdays.includes(weekday) ? weekdays.filter((item) => item !== weekday) : sortWeekdays([...weekdays, weekday])
}

export function sameRecurrence(left: RecurrenceFormState, right: RecurrenceFormState): boolean {
  return JSON.stringify(buildRecurrenceRule(left)) === JSON.stringify(buildRecurrenceRule(right))
}

export interface RecurrenceDescriptionInput {
  frequency: RecurrenceFrequency
  interval?: number | null
  by_weekday?: RecurrenceWeekday[] | null
  month_day?: number | null
  until?: string | null
  count?: number | null
}

function formatIsoDate(value: string, locale: string): string {
  const [year, month, day] = value.split('-').map(Number)
  if (!year || !month || !day) return value
  return new Intl.DateTimeFormat(locale, { timeZone: 'UTC', dateStyle: 'short' }).format(
    new Date(Date.UTC(year, month - 1, day)),
  )
}

export function describeRecurrence(
  rule: RecurrenceDescriptionInput,
  t: TFunction<'tasks'>,
  locale: string,
): string {
  const interval = rule.interval && rule.interval > 1 ? rule.interval : 1
  let text = t(`recurrence.describe.${rule.frequency}`, { count: interval })
  const weekdays = rule.by_weekday ?? []
  if (rule.frequency === 'weekly' && weekdays.length > 0) {
    text += ` ${t('recurrence.describe.onWeekdays', {
      days: sortWeekdays(weekdays)
        .map((weekday) => t(`recurrence.weekdaysShort.${weekday}`))
        .join(', '),
    })}`
  }
  if (rule.frequency === 'monthly' && rule.month_day) {
    text += ` ${t('recurrence.describe.onMonthDay', { day: rule.month_day })}`
  }
  if (rule.until) text += `, ${t('recurrence.describe.until', { date: formatIsoDate(rule.until, locale) })}`
  else if (rule.count) text += `, ${t('recurrence.describe.count', { count: rule.count })}`
  return text
}

export function reminderLabel(minutes: number | null, t: TFunction<'tasks'>): string {
  if (minutes === null) return t('reminder.none')
  if (minutes === 0) return t('reminder.atTime')
  if (minutes % 1440 === 0) return t('reminder.days', { count: minutes / 1440 })
  if (minutes % 60 === 0) return t('reminder.hours', { count: minutes / 60 })
  return t('reminder.minutes', { count: minutes })
}
