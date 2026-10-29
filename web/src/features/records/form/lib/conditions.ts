import type { FieldType, FilterNode } from '../../types'

export const CURRENT_USER = 'me'

export interface ConditionContext {
  now: Date
  timeZone: string
  userId: string
  fieldTypes: Record<string, FieldType>
}

export interface RuleTarget {
  visible_when?: FilterNode | null
  required_when?: FilterNode | null
}

export interface RuleState {
  visible: boolean
  required: boolean
}

type Values = Record<string, unknown>
type Range = { from: string | null; until: string | null }

const DATE = /^(\d{4})-(\d{2})-(\d{2})$/
const NEGATIONS: Record<string, string> = {
  neq: 'eq',
  not_contains: 'contains',
  not_in: 'in',
  is_empty: 'is_not_empty',
}

export function hasValue(value: unknown): boolean {
  if (value === null || value === undefined) return false
  if (typeof value === 'string') return value.trim() !== ''
  if (Array.isArray(value)) return value.length > 0
  return true
}

export function conditionMatches(filter: FilterNode, values: Values, context: ConditionContext): boolean {
  if ('op' in filter && 'conditions' in filter) {
    const children = filter.conditions
    if (filter.op === 'and') return children.every((child) => conditionMatches(child, values, context))
    return children.length === 0 || children.some((child) => conditionMatches(child, values, context))
  }
  if ('related' in filter) return false
  return leafMatches(filter.field, String(filter.operator), filter.value ?? null, values, context)
}

export function ruleState(target: RuleTarget, values: Values, context: ConditionContext): RuleState {
  const visible = !target.visible_when || conditionMatches(target.visible_when, values, context)
  const required = visible && Boolean(target.required_when) && conditionMatches(target.required_when!, values, context)
  return { visible, required }
}

function leafMatches(field: string, operator: string, operand: unknown, values: Values, context: ConditionContext): boolean {
  const fieldType = context.fieldTypes[field]
  if (!fieldType || !(field in values)) return false
  const value = values[field]
  if (fieldType === 'checkbox' && (operator === 'eq' || operator === 'neq')) {
    if (typeof operand !== 'boolean') return false
    const matchesExpected = (value === true) === operand
    return matchesExpected === (operator === 'eq')
  }
  const positive = NEGATIONS[operator] ?? operator
  const negated = positive !== operator
  return positiveMatches(positive, fieldType, value, operand, context) !== negated
}

function positiveMatches(
  operator: string,
  fieldType: FieldType,
  value: unknown,
  operand: unknown,
  context: ConditionContext,
): boolean {
  if (!hasValue(value)) return false
  switch (operator) {
    case 'is_not_empty':
      return true
    case 'eq':
      return equals(fieldType, value, operand, context)
    case 'in':
      return Array.isArray(operand) && operand.some((item) => equals(fieldType, value, item, context))
    case 'contains':
      return textTest(fieldType, value, operand, (text, needle) => text.includes(needle))
    case 'starts_with':
      return textTest(fieldType, value, operand, (text, needle) => text.startsWith(needle))
    case 'gt':
      return compare(fieldType, value, operand, context) === 1
    case 'gte': {
      const result = compare(fieldType, value, operand, context)
      return result === 1 || result === 0
    }
    case 'lt':
      return compare(fieldType, value, operand, context) === -1
    case 'lte': {
      const result = compare(fieldType, value, operand, context)
      return result === -1 || result === 0
    }
    case 'between': {
      if (!Array.isArray(operand) || operand.length !== 2) return false
      const low = compare(fieldType, value, operand[0], context)
      const high = compare(fieldType, value, operand[1], context)
      return (low === 1 || low === 0) && (high === -1 || high === 0)
    }
    default: {
      const range = relativeRange(operator, operand, context)
      if (!range) return false
      const day = localDay(fieldType, value, context)
      if (!day) return false
      return (range.from === null || day >= range.from) && (range.until === null || day < range.until)
    }
  }
}

function equals(fieldType: FieldType, value: unknown, operand: unknown, context: ConditionContext): boolean {
  if (Array.isArray(value)) return value.some((item) => equals(fieldType, item, operand, context))
  if (fieldType === 'number' || fieldType === 'currency' || fieldType === 'date' || fieldType === 'datetime') {
    return compare(fieldType, value, operand, context) === 0
  }
  if (fieldType === 'user' && typeof operand === 'string' && operand.trim() === CURRENT_USER) {
    return textOf(value) === context.userId.toLowerCase()
  }
  if (fieldType === 'phone') {
    const left = digitsOf(value)
    const right = digitsOf(operand)
    return left !== null && right !== null && samePhone(left, right)
  }
  const left = textOf(value)
  const right = textOf(operand)
  return left !== null && right !== null && left === right
}

const MIN_PHONE_SUFFIX_DIGITS = 8

function samePhone(left: string, right: string): boolean {
  const [shorter, longer] = left.length <= right.length ? [left, right] : [right, left]
  return shorter === longer || (shorter.length >= MIN_PHONE_SUFFIX_DIGITS && longer.endsWith(shorter))
}

function digitsOf(value: unknown): string | null {
  const digits = typeof value === 'string' ? value.replace(/\D/g, '') : typeof value === 'number' ? String(value) : ''
  return digits === '' ? null : digits
}

function sign(value: number): -1 | 0 | 1 {
  return value < 0 ? -1 : value > 0 ? 1 : 0
}

function compareText(left: string, right: string): -1 | 0 | 1 {
  return left < right ? -1 : left > right ? 1 : 0
}

function compare(fieldType: FieldType, value: unknown, operand: unknown, context: ConditionContext): -1 | 0 | 1 | null {
  if (fieldType === 'number' || fieldType === 'currency') {
    const left = numberOf(value)
    const right = numberOf(operand)
    return left === null || right === null ? null : sign(left - right)
  }
  if (fieldType === 'date') {
    const left = dateOf(value)
    const right = dateOf(operand)
    return left === null || right === null ? null : compareText(left, right)
  }
  if (fieldType === 'datetime') {
    const day = dateOf(operand)
    if (day !== null) {
      const local = localDay(fieldType, value, context)
      return local === null ? null : compareText(local, day)
    }
    const left = instantOf(value)
    const right = instantOf(operand)
    return left === null || right === null ? null : sign(left - right)
  }
  return null
}

function textOf(value: unknown): string | null {
  if (typeof value === 'string') return value.trim().normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase()
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  return null
}

function textTest(
  fieldType: FieldType,
  value: unknown,
  operand: unknown,
  test: (text: string, needle: string) => boolean,
): boolean {
  const comparable = fieldType === 'phone' ? digitsOf : textOf
  const needle = comparable(operand)
  if (!needle) return false
  const items = Array.isArray(value) ? value : [value]
  return items.some((item) => {
    const text = comparable(item)
    return text !== null && test(text, needle)
  })
}

function numberOf(value: unknown): number | null {
  const number = typeof value === 'number' ? value : typeof value === 'string' && value.trim() !== '' ? Number(value.trim()) : NaN
  return Number.isFinite(number) ? number : null
}

function dateOf(value: unknown): string | null {
  if (typeof value !== 'string') return null
  const text = value.trim()
  const match = DATE.exec(text)
  if (!match) return null
  const [year, month, day] = [Number(match[1]), Number(match[2]), Number(match[3])]
  const date = new Date(Date.UTC(year, month - 1, day))
  return date.getUTCFullYear() === year && date.getUTCMonth() === month - 1 && date.getUTCDate() === day ? text : null
}

function instantOf(value: unknown): number | null {
  if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}T/.test(value.trim())) return null
  const time = Date.parse(value.trim())
  return Number.isNaN(time) ? null : time
}

function zonedDay(instant: number, timeZone: string): string {
  const parts = new Intl.DateTimeFormat('en-CA', { timeZone, year: 'numeric', month: '2-digit', day: '2-digit' }).formatToParts(
    new Date(instant),
  )
  const part = (type: string) => parts.find((item) => item.type === type)?.value ?? ''
  return `${part('year')}-${part('month')}-${part('day')}`
}

function localDay(fieldType: FieldType, value: unknown, context: ConditionContext): string | null {
  if (fieldType === 'date') return dateOf(value)
  if (fieldType !== 'datetime') return null
  const instant = instantOf(value)
  return instant === null ? dateOf(value) : zonedDay(instant, context.timeZone)
}

function toUtc(day: string): Date {
  const [year, month, date] = day.split('-').map(Number)
  return new Date(Date.UTC(year, month - 1, date))
}

function format(date: Date): string {
  return date.toISOString().slice(0, 10)
}

function addDays(day: string, days: number): string {
  const date = toUtc(day)
  date.setUTCDate(date.getUTCDate() + days)
  return format(date)
}

function addMonths(day: string, months: number): string {
  const date = toUtc(day)
  return format(new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + months, 1)))
}

function relativeRange(operator: string, operand: unknown, context: ConditionContext): Range | null {
  const today = zonedDay(context.now.getTime(), context.timeZone)
  const weekday = (toUtc(today).getUTCDay() + 6) % 7
  const monday = addDays(today, -weekday)
  const first = `${today.slice(0, 8)}01`
  const year = Number(today.slice(0, 4))
  const days = typeof operand === 'number' && Number.isInteger(operand) && operand >= 0 ? operand : null
  const range = (from: string | null, until: string | null): Range => ({ from, until })
  switch (operator) {
    case 'today':
      return range(today, addDays(today, 1))
    case 'yesterday':
      return range(addDays(today, -1), today)
    case 'tomorrow':
      return range(addDays(today, 1), addDays(today, 2))
    case 'this_week':
      return range(monday, addDays(monday, 7))
    case 'last_week':
      return range(addDays(monday, -7), monday)
    case 'next_week':
      return range(addDays(monday, 7), addDays(monday, 14))
    case 'this_month':
      return range(first, addMonths(first, 1))
    case 'last_month':
      return range(addMonths(first, -1), first)
    case 'next_month':
      return range(addMonths(first, 1), addMonths(first, 2))
    case 'this_year':
      return range(`${year}-01-01`, `${year + 1}-01-01`)
    case 'within_last_days':
      return days === null ? null : range(addDays(today, -days), addDays(today, 1))
    case 'within_next_days':
      return days === null ? null : range(today, addDays(today, days + 1))
    case 'older_than_days':
      return days === null ? null : range(null, addDays(today, -days))
    default:
      return null
  }
}
