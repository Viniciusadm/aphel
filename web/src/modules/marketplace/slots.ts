import type { CatalogEntry } from './types'

declare module '@/kernel/modules/slots' {
  interface SlotPropsMap {
    'catalog.sections': { entries: CatalogEntry[] }
  }
}
