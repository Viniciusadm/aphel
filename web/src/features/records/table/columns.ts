import { activeFields } from '../fields'
import type { FieldDefinition, FieldSchema } from '../types'

export const DEFAULT_VISIBLE_COLUMNS = 6

export function defaultColumnKeys(schema: FieldSchema, titleField?: string, preferred?: string[]): string[] {
  const fields = activeFields(schema)
  const chosen = (preferred ?? []).filter((key) => fields.some((field) => field.key === key))
  if (chosen.length > 0) return chosen
  const title = fields.find((field) => field.key === titleField)
  const rest = fields.filter((field) => field !== title)
  return [...(title ? [title] : []), ...rest].slice(0, DEFAULT_VISIBLE_COLUMNS).map((field) => field.key)
}

export function resolveColumnFields(schema: FieldSchema, keys: string[], titleField?: string): FieldDefinition[] {
  const fields = activeFields(schema)
  const byKey = new Map(fields.map((field) => [field.key, field]))
  const resolved: FieldDefinition[] = []
  for (const key of keys) {
    const field = byKey.get(key)
    if (field && !resolved.includes(field)) resolved.push(field)
  }
  if (resolved.length > 0) return resolved
  return defaultColumnKeys(schema, titleField)
    .map((key) => byKey.get(key))
    .filter((field): field is FieldDefinition => Boolean(field))
}

export function moveKey(keys: string[], from: number, to: number): string[] {
  const next = [...keys]
  const [moved] = next.splice(from, 1)
  if (moved === undefined) return keys
  next.splice(to, 0, moved)
  return next
}
