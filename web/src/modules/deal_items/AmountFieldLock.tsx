import { useEffect } from 'react'
import type { SlotProps } from '@/kernel/modules/slots'
import { useDealItems } from './api'

const AMOUNT_FIELDS = ['amount']

export function AmountFieldLock({ dealId, lockFields }: SlotProps<'deals.detail.fieldLocks'>) {
  const items = useDealItems(dealId)
  const managed = items.data?.amount_managed === true

  useEffect(() => {
    lockFields('deal_items', managed ? AMOUNT_FIELDS : [])
  }, [lockFields, managed])

  return null
}
