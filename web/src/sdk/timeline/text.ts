import { type TimelineItem } from '../records'
export type TimelinePayload = Record<string, unknown>

export function asObject(value: unknown): TimelinePayload {
  return value !== null && typeof value === 'object' && !Array.isArray(value) ? (value as TimelinePayload) : {}
}

export function asText(value: unknown): string | null {
  if (typeof value === 'string' && value.trim() !== '') return value
  if (typeof value === 'number') return String(value)
  return null
}

export function payloadOf(item: TimelineItem): TimelinePayload {
  return asObject(item.metadata.payload)
}

export function eventType(item: TimelineItem): string {
  return asText(item.metadata.event_type) ?? item.title
}

export function nameOf(value: unknown): string | null {
  return asText(asObject(value).name) ?? asText(asObject(value).display_name) ?? asText(value)
}
