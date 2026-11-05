import { type TimelineItem } from '@/sdk'
import type { EmailAccess, EmailDirection, Participant, TimelineEmail } from '../types'

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function text(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value : null
}

function parseParticipant(value: unknown): Participant | null {
  if (!isRecord(value) || typeof value.address !== 'string') return null
  return {
    role: 'from',
    address: value.address,
    name: text(value.name),
    contact_id: text(value.contact_id),
  }
}

export function timelineEmail(item: TimelineItem): TimelineEmail | null {
  const raw = (item as TimelineItem & { email?: unknown }).email ?? item.metadata.email
  if (!isRecord(raw)) return null
  const threadId = text(raw.thread_id)
  if (!threadId) return null
  const access: EmailAccess = raw.access === 'full' ? 'full' : 'metadata'
  const direction: EmailDirection = raw.direction === 'outbound' ? 'outbound' : 'inbound'
  return {
    thread_id: threadId,
    subject: text(raw.subject),
    direction,
    access,
    snippet: access === 'full' ? text(raw.snippet) : null,
    from: parseParticipant(raw.from),
  }
}
