import { useTranslation } from 'react-i18next'

const MAX_LISTED = 8
const NAME_KEYS = ['label', 'name', 'key', 'value'] as const

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function isPrimitive(value: unknown): value is string | number | boolean {
  return typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean'
}

function displayName(value: Record<string, unknown>): string | null {
  for (const key of NAME_KEYS) {
    const candidate = value[key]
    if (typeof candidate === 'string' && candidate) return candidate
  }
  return null
}

export function isStructured(value: unknown): boolean {
  return (Array.isArray(value) && value.some((entry) => !isPrimitive(entry))) || (isRecord(value) && Object.keys(value).length > 0)
}

function countSections(tabs: unknown[]): number {
  return tabs.reduce<number>((total, tab) => {
    const sections = isRecord(tab) ? tab.sections : undefined
    return total + (Array.isArray(sections) ? sections.length : 0)
  }, 0)
}

export function useValueDescriber() {
  const { t } = useTranslation('config-packages')

  const primitive = (value: string | number | boolean): string => {
    if (typeof value === 'boolean') return t(value ? 'diff.yes' : 'diff.no')
    if (value === '') return t('diff.emptyText')
    return String(value)
  }

  const list = (names: string[]): string => {
    if (names.length === 0) return t('diff.emptyList')
    const shown = names.slice(0, MAX_LISTED).join(', ')
    const hidden = names.length - MAX_LISTED
    return hidden > 0 ? t('diff.listMore', { list: shown, count: hidden }) : shown
  }

  const describe = (value: unknown): string => {
    if (value === null || value === undefined) return t('diff.none')
    if (isPrimitive(value)) return primitive(value)
    if (Array.isArray(value)) {
      const names = value.map((entry) => (isPrimitive(entry) ? primitive(entry) : isRecord(entry) ? displayName(entry) : null))
      if (names.every((name): name is string => name !== null)) return list(names)
      return t('diff.items', { count: value.length })
    }
    if (isRecord(value)) {
      if (Array.isArray(value.tabs)) {
        return t('diff.layout', {
          tabs: t('diff.tabs', { count: value.tabs.length }),
          sections: t('diff.sections', { count: countSections(value.tabs) }),
        })
      }
      const name = displayName(value)
      if (name) return value.active === false ? t('diff.inactiveName', { name }) : name
      const keys = Object.keys(value).length
      return keys === 0 ? t('diff.emptyObject') : t('diff.properties', { count: keys })
    }
    return String(value)
  }

  return describe
}
