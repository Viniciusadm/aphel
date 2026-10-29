import type { components } from '@/api'

type Schemas = components['schemas']

export type CatalogEntryKind = Schemas['CatalogEntryKindResponse']
export type CatalogPackage = Schemas['CatalogPackageResponse']
export type CatalogModule = Schemas['CatalogModuleResponse']
export type CatalogEntry = Schemas['CatalogEntryResponse']
export type RejectedCatalogEntry = Schemas['RejectedCatalogEntryResponse']
export type Catalog = Schemas['CatalogResponse']
