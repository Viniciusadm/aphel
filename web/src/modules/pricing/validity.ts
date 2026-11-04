import type { PriceList } from './types'

function today(): string {
  const now = new Date()
  const month = String(now.getMonth() + 1).padStart(2, '0')
  const day = String(now.getDate()).padStart(2, '0')
  return `${now.getFullYear()}-${month}-${day}`
}

export function isInEffect(list: PriceList, on = today()): boolean {
  return (
    list.is_active &&
    (list.valid_from == null || list.valid_from <= on) &&
    (list.valid_to == null || list.valid_to >= on)
  )
}
