import type { CompanyHierarchyNode, CompanyTreeNode } from './types'

export function buildCompanyTree(nodes: CompanyHierarchyNode[]): CompanyTreeNode[] {
  const byId = new Map<string, CompanyTreeNode>()
  for (const node of nodes) byId.set(node.id, { node, children: [] })
  const roots: CompanyTreeNode[] = []
  for (const node of nodes) {
    const entry = byId.get(node.id)
    if (!entry) continue
    const parent = node.parent_id ? byId.get(node.parent_id) : undefined
    if (parent) parent.children.push(entry)
    else roots.push(entry)
  }
  return roots
}

export function ancestorIds(nodes: CompanyHierarchyNode[], companyId: string): Set<string> {
  const parents = new Map(nodes.map((node) => [node.id, node.parent_id]))
  const ancestors = new Set<string>()
  let cursor = parents.get(companyId) ?? null
  while (cursor && !ancestors.has(cursor)) {
    ancestors.add(cursor)
    cursor = parents.get(cursor) ?? null
  }
  return ancestors
}

export function groupFilterSearch(companyId: string): string {
  return new URLSearchParams({ 'filter[company_id][in_group]': companyId }).toString()
}
