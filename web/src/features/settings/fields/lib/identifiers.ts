import { KEY_MAX_LENGTH } from './field-types'

export const KEY_PATTERN = /^[a-z](?:_?[a-z0-9])*$/

const FALLBACK_OPTION_VALUE = 'option'

function slugify(label: string): string {
  const normalized = label.trim().toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '')
  let slug = ''
  for (const character of normalized) {
    const mapped = /[a-z0-9]/.test(character) ? character : '_'
    if (mapped === '_' && (slug === '' || slug.endsWith('_'))) continue
    slug += mapped
  }
  return slug.replace(/_+$/, '')
}

function identifierFrom(label: string, digitPrefix: string): string {
  const slug = slugify(label)
  const prefixed = /^[0-9]/.test(slug) ? `${digitPrefix}_${slug}` : slug
  return prefixed.slice(0, KEY_MAX_LENGTH).replace(/_+$/, '')
}

export function isValidKey(value: string): boolean {
  return value.length <= KEY_MAX_LENGTH && KEY_PATTERN.test(value)
}

export function keyFromLabel(label: string): string {
  return identifierFrom(label, 'field')
}

export function optionValuesFromLabels(labels: string[]): string[] {
  const taken: string[] = []
  return labels.map((label) => {
    const base = identifierFrom(label, FALLBACK_OPTION_VALUE) || FALLBACK_OPTION_VALUE
    let candidate = base
    let suffix = 2
    while (taken.includes(candidate)) {
      candidate = `${base}_${suffix}`
      suffix += 1
    }
    taken.push(candidate)
    return candidate
  })
}
