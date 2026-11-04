import { z } from 'zod'
import { isValidTimeZone } from '../lib/format'
import type { BookingAvailabilityRequest, BookingAvailabilityResponse } from '../types'

export const MAX_INTERVALS = 28
export const MAX_NOTICE_MINUTES = 43_200
export const MAX_PER_DAY = 50
export const MAX_BUFFER_MINUTES = 240
export const MAX_HORIZON_DAYS = 365

const TIME_PATTERN = /^([01]\d|2[0-3]):[0-5]\d$|^24:00$/

export function timeToMinutes(value: string): number {
  const [hours, minutes] = value.split(':').map(Number)
  return hours * 60 + minutes
}

const intervalSchema = z
  .object({
    weekday: z.number().int().min(1).max(7),
    start: z.string().regex(TIME_PATTERN, 'validation.time'),
    end: z.string().regex(TIME_PATTERN, 'validation.time'),
  })
  .refine(
    (interval) =>
      !TIME_PATTERN.test(interval.start) ||
      !TIME_PATTERN.test(interval.end) ||
      timeToMinutes(interval.end) > timeToMinutes(interval.start),
    { message: 'validation.intervalOrder', path: ['end'] },
  )

function integerIn(min: number, max: number, message: string) {
  return z.number({ error: message }).int(message).min(min, message).max(max, message)
}

export const availabilityFormSchema = z
  .object({
    timezone: z.string().trim().min(1, 'validation.timezone').refine(isValidTimeZone, 'validation.timezone'),
    intervals: z.array(intervalSchema).max(MAX_INTERVALS, 'validation.tooManyIntervals'),
    minNoticeMinutes: integerIn(0, MAX_NOTICE_MINUTES, 'validation.minNotice'),
    maxPerDay: z
      .string()
      .trim()
      .refine((value) => {
        if (!value) return true
        const parsed = Number(value)
        return Number.isInteger(parsed) && parsed >= 1 && parsed <= MAX_PER_DAY
      }, 'validation.maxPerDay'),
    bufferBeforeMinutes: integerIn(0, MAX_BUFFER_MINUTES, 'validation.buffer'),
    bufferAfterMinutes: integerIn(0, MAX_BUFFER_MINUTES, 'validation.buffer'),
    horizonDays: integerIn(1, MAX_HORIZON_DAYS, 'validation.horizon'),
  })
  .superRefine((values, context) => {
    values.intervals.forEach((interval, index) => {
      if (!TIME_PATTERN.test(interval.start) || !TIME_PATTERN.test(interval.end)) return
      const start = timeToMinutes(interval.start)
      const end = timeToMinutes(interval.end)
      const overlaps = values.intervals.some((other, otherIndex) => {
        if (otherIndex >= index || other.weekday !== interval.weekday) return false
        if (!TIME_PATTERN.test(other.start) || !TIME_PATTERN.test(other.end)) return false
        return start < timeToMinutes(other.end) && timeToMinutes(other.start) < end
      })
      if (overlaps) {
        context.addIssue({ code: 'custom', message: 'validation.overlap', path: ['intervals', index, 'start'] })
      }
    })
  })

export type AvailabilityFormValues = z.infer<typeof availabilityFormSchema>
export type IntervalFormValues = AvailabilityFormValues['intervals'][number]

export function defaultValues(availability: BookingAvailabilityResponse): AvailabilityFormValues {
  return {
    timezone: availability.timezone,
    intervals: availability.weekly_hours.map((interval) => ({ ...interval })),
    minNoticeMinutes: availability.min_notice_minutes,
    maxPerDay: availability.max_per_day == null ? '' : String(availability.max_per_day),
    bufferBeforeMinutes: availability.buffer_before_minutes,
    bufferAfterMinutes: availability.buffer_after_minutes,
    horizonDays: availability.horizon_days,
  }
}

export function toRequest(values: AvailabilityFormValues): BookingAvailabilityRequest {
  const intervals = [...values.intervals].sort(
    (left, right) => left.weekday - right.weekday || timeToMinutes(left.start) - timeToMinutes(right.start),
  )
  return {
    timezone: values.timezone.trim(),
    weekly_hours: intervals,
    min_notice_minutes: values.minNoticeMinutes,
    max_per_day: values.maxPerDay.trim() ? Number(values.maxPerDay) : null,
    buffer_before_minutes: values.bufferBeforeMinutes,
    buffer_after_minutes: values.bufferAfterMinutes,
    horizon_days: values.horizonDays,
  }
}

export const serverFieldPaths = {
  timezone: 'timezone',
  weekly_hours: 'intervals',
  min_notice_minutes: 'minNoticeMinutes',
  max_per_day: 'maxPerDay',
  buffer_before_minutes: 'bufferBeforeMinutes',
  buffer_after_minutes: 'bufferAfterMinutes',
  horizon_days: 'horizonDays',
} as const satisfies Record<string, keyof AvailabilityFormValues>
