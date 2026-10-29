import { useState } from 'react'

function sameItems<T>(left: T[], right: T[]): boolean {
  return left.length === right.length && left.every((item, index) => item === right[index])
}

export function useStableList<T>(items: T[]): T[] {
  const [stable, setStable] = useState(items)
  if (sameItems(stable, items)) return stable
  setStable(items)
  return items
}
