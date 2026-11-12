import type { DocumentRecord } from './document'

export type FieldChange =
  | { field: string; kind: 'text'; before: string; after: string }
  | { field: string; kind: 'blocks'; added: number; removed: number; changed: number }
  | { field: string; kind: 'languages'; languages: string[] }
  | { field: string; kind: 'list'; added: number; removed: number }

function text(value: unknown): string {
  if (value === null || value === undefined) return ''
  return typeof value === 'string' ? value : String(value)
}

function blocksOf(value: unknown): Record<string, string> {
  try {
    const parsed = JSON.parse(text(value) || '[]') as { id?: string }[]
    return Object.fromEntries(parsed.map((block, index) => [block.id ?? `#${index}`, JSON.stringify(block)]))
  } catch {
    return {}
  }
}

function mapOf(value: unknown): Record<string, string> {
  try {
    const parsed = JSON.parse(text(value) || '{}') as Record<string, unknown>
    return Object.fromEntries(Object.entries(parsed).map(([key, item]) => [key, text(item)]))
  } catch {
    return {}
  }
}

function isDocument(value: unknown): boolean {
  return typeof value === 'string' && value.trim().startsWith('[')
}

export function revisionChanges(fields: Record<string, unknown>, record: DocumentRecord): FieldChange[] {
  const changes: FieldChange[] = []
  for (const [field, before] of Object.entries(fields)) {
    const after = record[field]
    if (field.endsWith('_translations')) {
      const old = mapOf(before)
      const now = mapOf(after)
      const languages = [...new Set([...Object.keys(old), ...Object.keys(now)])].filter((language) => (old[language] ?? '') !== (now[language] ?? ''))
      if (languages.length) changes.push({ field, kind: 'languages', languages })
      continue
    }
    if (Array.isArray(before) || Array.isArray(after)) {
      const old = new Set((Array.isArray(before) ? before : []).map(String))
      const now = new Set((Array.isArray(after) ? after : []).map(String))
      const added = [...now].filter((item) => !old.has(item)).length
      const removed = [...old].filter((item) => !now.has(item)).length
      if (added || removed) changes.push({ field, kind: 'list', added, removed })
      continue
    }
    if (isDocument(before) || isDocument(after)) {
      const old = blocksOf(before)
      const now = blocksOf(after)
      const added = Object.keys(now).filter((id) => !(id in old)).length
      const removed = Object.keys(old).filter((id) => !(id in now)).length
      const changed = Object.keys(now).filter((id) => id in old && old[id] !== now[id]).length
      if (added || removed || changed) changes.push({ field, kind: 'blocks', added, removed, changed })
      continue
    }
    if (text(before) !== text(after)) changes.push({ field, kind: 'text', before: text(before), after: text(after) })
  }
  return changes
}
