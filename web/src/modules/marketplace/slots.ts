import type {CatalogEntry} from '@/sdk'

declare module '@/kernel/modules/slots' {
  interface SlotPropsMap {
    'catalog.sections': { entries: CatalogEntry[] }
  }
}
