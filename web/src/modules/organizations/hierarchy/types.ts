import type { components } from '@/api'

type Schemas = components['schemas']

export type CompanyHierarchyOwner = Schemas['UserReferenceResponse']
export type CompanyHierarchyNode = Schemas['CompanyHierarchyNodeResponse']
export type CompanyHierarchy = Schemas['CompanyHierarchyResponse']
export type CompanyGroupTotals = Schemas['CompanyGroupTotalsResponse']

export interface CompanyTreeNode {
  node: CompanyHierarchyNode
  children: CompanyTreeNode[]
}
