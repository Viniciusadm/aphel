import { utcToZonedInput, zonedInputToUtc } from '@/components/fields/lib/datetime'

export type ReminderPreset = 'hour' | 'tomorrow' | 'week'

export const REMINDER_PRESETS: ReminderPreset[] = ['hour', 'tomorrow', 'week']

function toIso(date: Date): string {
  return date.toISOString().replace(/\.\d{3}Z$/, 'Z')
}

function morningAfter(days: number, timeZone: string): string | null {
  const target = new Date(Date.now() + days * 86_400_000)
  const day = utcToZonedInput(toIso(target), timeZone).slice(0, 10)
  return zonedInputToUtc(`${day}T09:00`, timeZone)
}

export function presetInstant(preset: ReminderPreset, timeZone: string): string | null {
  if (preset === 'hour') return toIso(new Date(Date.now() + 3_600_000))
  return morningAfter(preset === 'tomorrow' ? 1 : 7, timeZone)
}
