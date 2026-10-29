import { arrayMove } from '@dnd-kit/sortable'

export function moveId(ids: string[], from: number, to: number): string[] {
  return arrayMove(ids, from, to)
}

export function mergeVisibleOrder<T extends { id: string }>(items: T[], visibleOrder: string[]): string[] {
  const visible = new Set(visibleOrder)
  let next = 0
  return items.map((item) => {
    if (!visible.has(item.id)) return item.id
    const id = visibleOrder[next] ?? item.id
    next += 1
    return id
  })
}
