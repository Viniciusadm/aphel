import type { components } from '@/modules/organizations/api-types'

type Schemas = components['schemas']

export type CompanyHierarchyOwner = Schemas['UserReferenceResponse']
export type CompanyHierarchyNode = Schemas['CompanyHierarchyNodeResponse']
export type CompanyHierarchy = Schemas['CompanyHierarchyResponse']
export type CompanyGroupTotals = Schemas['CompanyGroupTotalsResponse']

export interface CompanyTreeNode {
  node: CompanyHierarchyNode
  children: CompanyTreeNode[]
}
