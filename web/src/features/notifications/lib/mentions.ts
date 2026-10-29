export const MENTION_PATTERN = /@\[([^[\]\n]{1,200})\]\(user:([0-9a-fA-F-]{36})\)/g

export interface MentionQuery {
  start: number
  end: number
  query: string
}

const TRIGGER = /(^|\s)@([^\s@[\]()]{0,40})$/

export function findMentionQuery(value: string, caret: number): MentionQuery | null {
  const before = value.slice(0, caret)
  const match = TRIGGER.exec(before)
  if (!match) return null
  const query = match[2]
  return { start: caret - query.length - 1, end: caret, query }
}

export function mentionMarkup(name: string, userId: string): string {
  const clean = name.replace(/[[\]\n]/g, '').trim()
  return `@[${clean}](user:${userId})`
}

export function insertMention(value: string, mention: MentionQuery, name: string, userId: string) {
  const markup = `${mentionMarkup(name, userId)} `
  const next = value.slice(0, mention.start) + markup + value.slice(mention.end)
  return { value: next, caret: mention.start + markup.length }
}

export function mentionsToText(body: string): string {
  return body.replace(MENTION_PATTERN, (_match, name: string) => `@${name}`)
}

export function mentionedUserIds(body: string): string[] {
  const ids = new Set<string>()
  for (const match of body.matchAll(MENTION_PATTERN)) ids.add(match[2].toLowerCase())
  return [...ids]
}
