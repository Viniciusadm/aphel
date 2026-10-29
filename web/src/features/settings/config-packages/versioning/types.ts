import type { components } from '@/api'

type Schemas = components['schemas']

export type PackageStatus = Schemas['PackageStatusResponse']
export type VersionRelation = Schemas['VersionRelationResponse']
export type Resolution = Schemas['ResolutionBody']
export type Resolutions = Record<string, Resolution>

export type PackageIdentity = Schemas['PackageIdentityResponse']
export type InstalledVersion = Schemas['InstalledVersionResponse']
export type VersionItem = Schemas['VersionItemResponse']
export type ComparisonSummary = Schemas['ComparisonSummaryResponse']
export type VersionComparison = Schemas['VersionComparisonResponse']
export type VersionApplyResult = Schemas['VersionApplyResponse']
