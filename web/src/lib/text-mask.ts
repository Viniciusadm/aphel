import { maskNumber, numberToCanonical, numberToDisplay } from './number-mask'

export const TEXT_MASK_KINDS = [
  'cpf',
  'cnpj',
  'cpf_cnpj',
  'cep',
  'uf',
  'ibge_city',
  'ibge_state',
  'ncm',
  'cest',
  'cfop',
  'gtin',
  'time',
  'digits',
  'percent',
] as const

export type TextMaskKind = (typeof TEXT_MASK_KINDS)[number]

const PATTERNS: Partial<Record<TextMaskKind, string>> = {
  cpf: '000.000.000-00',
  cnpj: 'AA.AAA.AAA/AAAA-00',
  cep: '00000-000',
  ibge_city: '0000000',
  ibge_state: '00',
  ncm: '0000.00.00',
  cest: '00.000.00',
  cfop: '0.000',
  uf: 'LL',
  time: '00:00',
}

const GTIN_LENGTHS = [8, 12, 13, 14]
const GTIN_MAX_LENGTH = 14

const BRAZILIAN_STATES = new Set([
  'AC', 'AL', 'AP', 'AM', 'BA', 'CE', 'DF', 'ES', 'GO', 'MA', 'MT', 'MS', 'MG', 'PA', 'PB', 'PR',
  'PE', 'PI', 'RJ', 'RN', 'RS', 'RO', 'RR', 'SC', 'SP', 'SE', 'TO',
])

const CNPJ_FIRST_WEIGHTS = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]
const CNPJ_SECOND_WEIGHTS = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]

export function isTextMaskKind(value: unknown): value is TextMaskKind {
  return typeof value === 'string' && (TEXT_MASK_KINDS as readonly string[]).includes(value)
}

function isSlot(slot: string): boolean {
  return slot === '0' || slot === 'A' || slot === 'L'
}

function fitsSlot(character: string, slot: string): boolean {
  if (slot === '0') return /\d/.test(character)
  if (slot === 'L') return /[A-Z]/.test(character)
  return /[0-9A-Z]/.test(character)
}

function patternSlots(pattern: string): string[] {
  return [...pattern].filter(isSlot)
}

function fitToSlots(raw: string, slots: string[]): string {
  let result = ''
  for (const character of raw.toUpperCase()) {
    if (result.length >= slots.length) break
    if (fitsSlot(character, slots[result.length])) result += character
  }
  return result
}

function applyPattern(canonical: string, pattern: string): string {
  let display = ''
  let index = 0
  for (const slot of pattern) {
    if (index >= canonical.length) break
    if (isSlot(slot)) {
      display += canonical[index]
      index += 1
    } else {
      display += slot
    }
  }
  return display
}

function cpfCnpjPattern(canonical: string): string {
  return canonical.length <= 11 && /^\d*$/.test(canonical) ? (PATTERNS.cpf as string) : (PATTERNS.cnpj as string)
}

export function cleanTextMask(raw: string, kind: TextMaskKind): string {
  switch (kind) {
    case 'cpf_cnpj': {
      const characters = raw.toUpperCase().replace(/[^0-9A-Z]/g, '')
      return fitToSlots(characters, patternSlots(cpfCnpjPattern(characters.slice(0, 12))))
    }
    case 'time':
      return applyPattern(fitToSlots(raw, patternSlots(PATTERNS.time as string)), PATTERNS.time as string)
    case 'gtin':
      return raw.replace(/\D/g, '').slice(0, GTIN_MAX_LENGTH)
    case 'digits':
      return raw.replace(/\D/g, '')
    case 'percent':
      return numberToCanonical(maskNumber(raw, 'percent', 'comma'))
    default:
      return fitToSlots(raw, patternSlots(PATTERNS[kind] as string))
  }
}

export function formatTextMask(canonical: string, kind: TextMaskKind): string {
  switch (kind) {
    case 'cpf_cnpj':
      return applyPattern(canonical, cpfCnpjPattern(canonical))
    case 'time':
    case 'gtin':
    case 'digits':
      return canonical
    case 'percent':
      return numberToDisplay(canonical, 'comma')
    default:
      return applyPattern(canonical, PATTERNS[kind] as string)
  }
}

export function formatStoredTextMask(stored: string, kind: TextMaskKind): string {
  return formatTextMask(cleanTextMask(stored, kind), kind)
}

export function textMaskInputMode(kind: TextMaskKind): 'numeric' | 'decimal' | 'text' {
  if (kind === 'percent') return 'decimal'
  if (kind === 'cnpj' || kind === 'cpf_cnpj' || kind === 'uf') return 'text'
  return 'numeric'
}

export function textMaskMaxLength(kind: TextMaskKind): number | undefined {
  if (kind === 'cpf_cnpj') return (PATTERNS.cnpj as string).length
  if (kind === 'gtin') return GTIN_MAX_LENGTH
  const pattern = PATTERNS[kind]
  return pattern ? pattern.length : undefined
}

export function textMaskPlaceholder(kind: TextMaskKind): string | undefined {
  switch (kind) {
    case 'cpf_cnpj':
      return '000.000.000-00'
    case 'cnpj':
      return '00.000.000/0000-00'
    case 'uf':
      return 'SP'
    case 'time':
      return '00:00'
    case 'gtin':
    case 'digits':
    case 'percent':
      return undefined
    default:
      return PATTERNS[kind]
  }
}

function repeated(code: string): boolean {
  return /^(.)\1+$/.test(code)
}

function isValidCpf(code: string): boolean {
  if (!/^\d{11}$/.test(code) || repeated(code)) return false
  const values = [...code].map(Number)
  const check = (length: number) => {
    const sum = values.slice(0, length).reduce((total, value, index) => total + value * (length + 1 - index), 0)
    const remainder = (sum * 10) % 11
    return remainder === 10 ? 0 : remainder
  }
  return check(9) === values[9] && check(10) === values[10]
}

function cnpjCheckDigit(values: number[], weights: number[]): number {
  const sum = values.reduce((total, value, index) => total + value * weights[index], 0)
  const remainder = sum % 11
  return remainder < 2 ? 0 : 11 - remainder
}

function isValidCnpj(code: string): boolean {
  if (!/^[0-9A-Z]{12}\d{2}$/.test(code) || repeated(code)) return false
  const values = [...code].map((character) => character.charCodeAt(0) - 48)
  return (
    values[12] === cnpjCheckDigit(values.slice(0, 12), CNPJ_FIRST_WEIGHTS) &&
    values[13] === cnpjCheckDigit(values.slice(0, 13), CNPJ_SECOND_WEIGHTS)
  )
}

export function normalizeCnpj(raw: string): string | null {
  const code = cleanTextMask(raw, 'cnpj')
  return isValidCnpj(code) ? code : null
}

function isValidTime(value: string): boolean {
  const match = /^(\d{2}):(\d{2})$/.exec(value)
  if (!match) return false
  const hours = Number(match[1])
  const minutes = Number(match[2])
  return (hours < 24 && minutes < 60) || (hours === 24 && minutes === 0)
}

export function isTextMaskValid(value: string, kind: TextMaskKind): boolean {
  const canonical = cleanTextMask(value, kind)
  switch (kind) {
    case 'cpf':
      return isValidCpf(canonical)
    case 'cnpj':
      return isValidCnpj(canonical)
    case 'cpf_cnpj':
      return canonical.length === 11 ? isValidCpf(canonical) : isValidCnpj(canonical)
    case 'uf':
      return BRAZILIAN_STATES.has(canonical)
    case 'cfop':
      return /^[1-7]\d{3}$/.test(canonical)
    case 'gtin':
      return GTIN_LENGTHS.includes(canonical.length)
    case 'time':
      return isValidTime(canonical)
    case 'digits':
      return canonical.length > 0
    case 'percent': {
      const number = Number(canonical)
      return canonical !== '' && Number.isFinite(number) && number >= 0 && number <= 100
    }
    default:
      return canonical.length === patternSlots(PATTERNS[kind] as string).length
  }
}
