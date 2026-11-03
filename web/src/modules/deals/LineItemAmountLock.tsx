import { useIsModuleActive } from '@/kernel/modules/registry'
import { useEffect } from 'react'
import type { SlotProps } from '@/kernel/modules/slots'
import { useLineItems } from '@/sdk/line-items'

const AMOUNT_FIELDS = ['amount']

export function LineItemAmountLock({ dealId, lockFields }: SlotProps<'deals.detail.fieldLocks'>) {
  const enabled = useIsModuleActive('line_items')
  const items = useLineItems('deals.deal', dealId, enabled)
  const managed = items.data?.amount_managed === true

  useEffect(() => {
    lockFields('line_items', managed ? AMOUNT_FIELDS : [])
  }, [lockFields, managed])

  return null
}
