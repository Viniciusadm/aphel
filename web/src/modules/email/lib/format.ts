import type { EmailAddress, EmailMessage, EmailThread, Participant, ParticipantRole } from '../types'

const ADDRESS_PATTERN = /^[^\s@<>(),;:"]+@[^\s@<>(),;:"]+\.[^\s@<>(),;:"]+$/

export function isValidAddress(value: string): boolean {
  return ADDRESS_PATTERN.test(value.trim())
}

export function normalizeAddress(value: string): string {
  return value.trim().toLowerCase()
}

export function participantName(participant: Pick<Participant, 'address' | 'name'>): string {
  return participant.name?.trim() || participant.address
}

export function participantsByRole(participants: Participant[], ...roles: ParticipantRole[]): Participant[] {
  return participants.filter((participant) => roles.includes(participant.role))
}

export function joinParticipants(participants: Participant[], limit = 3): string {
  const names = participants.map(participantName)
  if (names.length <= limit) return names.join(', ')
  return `${names.slice(0, limit).join(', ')} +${names.length - limit}`
}

export function distinctParticipants(participants: Participant[]): Participant[] {
  const seen = new Set<string>()
  return participants.filter((participant) => {
    const key = normalizeAddress(participant.address)
    if (seen.has(key)) return false
    seen.add(key)
    return true
  })
}

export function threadCounterparts(thread: EmailThread): Participant[] {
  const own = normalizeAddress(thread.mailbox_address)
  return distinctParticipants(thread.participants.filter((participant) => normalizeAddress(participant.address) !== own))
}

export function toEmailAddress(participant: Pick<Participant, 'address' | 'name'>): EmailAddress {
  return participant.name ? { address: participant.address, name: participant.name } : { address: participant.address }
}

export function replyRecipients(message: EmailMessage, ownAddress: string): EmailAddress[] {
  const own = normalizeAddress(ownAddress)
  const replyTo = participantsByRole(message.participants, 'reply_to')
  const source =
    message.direction === 'inbound'
      ? replyTo.length > 0
        ? replyTo
        : participantsByRole(message.participants, 'from')
      : participantsByRole(message.participants, 'to')
  return distinctParticipants(source.filter((participant) => normalizeAddress(participant.address) !== own)).map(
    toEmailAddress,
  )
}

export function replySubject(subject: string | null): string {
  const base = subject?.trim() ?? ''
  return /^re:/i.test(base) ? base : `Re: ${base}`.trim()
}

export function formatFileSize(bytes: number, locale: string): string {
  const units = ['B', 'KB', 'MB', 'GB']
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  const digits = unit === 0 || value >= 10 ? 0 : 1
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: digits }).format(value)} ${units[unit]}`
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

export function textToHtml(text: string): string {
  return text
    .split(/\n{2,}/)
    .map((paragraph) => `<p>${escapeHtml(paragraph).replace(/\n/g, '<br>')}</p>`)
    .join('\n')
}

export function slugifyKey(value: string): string {
  return value
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 64)
}

export function hasVariables(value: string): boolean {
  return /\{\{\s*[\w.]+\s*\}\}/.test(value)
}

export function insertAt(value: string, insertion: string, start: number | null, end: number | null): string {
  const from = start ?? value.length
  const to = end ?? from
  return `${value.slice(0, from)}${insertion}${value.slice(to)}`
}
