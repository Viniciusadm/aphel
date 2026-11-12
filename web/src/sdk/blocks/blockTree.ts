import { duplicateBlock, isContainer, type Block } from './blocks'

export interface BlockLocation {
  parent: string | null
  index: number
}

export function findBlock(blocks: Block[], id: string): Block | null {
  for (const block of blocks) {
    if (block.id === id) return block
    if (isContainer(block)) {
      const found = findBlock(block.children, id)
      if (found) return found
    }
  }
  return null
}

export function locateBlock(blocks: Block[], id: string, parent: string | null = null): BlockLocation | null {
  for (const [index, block] of blocks.entries()) {
    if (block.id === id) return { parent, index }
    if (isContainer(block)) {
      const found = locateBlock(block.children, id, block.id)
      if (found) return found
    }
  }
  return null
}

function mapChildren(blocks: Block[], parent: string | null, change: (children: Block[]) => Block[]): Block[] {
  if (parent === null) return change(blocks)
  return blocks.map((block) => {
    if (!isContainer(block)) return block
    if (block.id === parent) return { ...block, children: change(block.children) }
    return { ...block, children: mapChildren(block.children, parent, change) }
  })
}

export function replaceBlock(blocks: Block[], next: Block): Block[] {
  return blocks.map((block) => {
    if (block.id === next.id) return next
    if (isContainer(block)) return { ...block, children: replaceBlock(block.children, next) }
    return block
  })
}

export function removeBlock(blocks: Block[], id: string): Block[] {
  return blocks
    .filter((block) => block.id !== id)
    .map((block) => (isContainer(block) ? { ...block, children: removeBlock(block.children, id) } : block))
}

export function insertBlock(blocks: Block[], parent: string | null, index: number, block: Block): Block[] {
  return mapChildren(blocks, parent, (children) => [...children.slice(0, index), block, ...children.slice(index)])
}

export function moveBlock(blocks: Block[], id: string, offset: number): Block[] {
  const location = locateBlock(blocks, id)
  if (!location) return blocks
  return mapChildren(blocks, location.parent, (children) => {
    const target = location.index + offset
    if (target < 0 || target >= children.length) return children
    const next = [...children]
    const [item] = next.splice(location.index, 1)
    next.splice(target, 0, item)
    return next
  })
}

export function duplicateInPlace(blocks: Block[], id: string): { blocks: Block[]; copy: Block | null } {
  const location = locateBlock(blocks, id)
  const original = findBlock(blocks, id)
  if (!location || !original) return { blocks, copy: null }
  const copy = duplicateBlock(original)
  return { blocks: insertBlock(blocks, location.parent, location.index + 1, copy), copy }
}
