import type { ParseKeys } from 'i18next'
import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useTrashTypes } from './api'

const SINGULAR: Record<string, ParseKeys<'trash'>> = {
  contact: 'recordType.contact',
  company: 'recordType.company',
  deal: 'recordType.deal',
  product: 'recordType.product',
  activity: 'recordType.activity',
  quote: 'recordType.quote',
}

const PLURAL: Record<string, ParseKeys<'trash'>> = {
  contact: 'recordTypes.contact',
  company: 'recordTypes.company',
  deal: 'recordTypes.deal',
  product: 'recordTypes.product',
  activity: 'recordTypes.activity',
  quote: 'recordTypes.quote',
}

export function useTrashTypeLabel() {
  const { t } = useTranslation('trash')
  const types = useTrashTypes()
  return useCallback(
    (recordType: string, plural = false): string => {
      const key = (plural ? PLURAL : SINGULAR)[recordType]
      if (key) return t(key)
      return types.data?.find((type) => type.record_type === recordType)?.label ?? recordType
    },
    [t, types.data],
  )
}
