import { useModuleTranslation } from '@/kernel/modules/i18n'
import { useContext } from 'react'

import { DocumentKindContext } from './document'

export type EditorialTranslate = (key: string, options?: Record<string, unknown>) => string

export function useEditorialTranslation(): EditorialTranslate {
  const base = useModuleTranslation('editorial') as unknown as EditorialTranslate
  const kind = useContext(DocumentKindContext)
  const own = useModuleTranslation(kind?.texts?.namespace ?? 'editorial') as unknown as EditorialTranslate
  return (key, options) => {
    if (kind?.texts) {
      const scoped = `${kind.texts.scope}.${key}`
      const text = own(scoped, options)
      if (text !== scoped) return text
    }
    return base(key, options)
  }
}
