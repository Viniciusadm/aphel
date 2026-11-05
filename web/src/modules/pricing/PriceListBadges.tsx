import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { formatDate, useRecordFormatting } from '@/sdk'
import type { PriceList } from './types'
import { isInEffect } from './validity'

export function PriceListBadges({ list }: { list: PriceList }) {
  const { t } = useTranslation('pricing')
  return (
    <>
      {list.is_default && <Badge tone="primary">{t('badges.default')}</Badge>}
      {!list.is_active ? (
        <Badge tone="neutral">{t('badges.inactive')}</Badge>
      ) : (
        !isInEffect(list) && <Badge tone="warning">{t('badges.outOfPeriod')}</Badge>
      )}
    </>
  )
}

export function PriceListValidity({ list }: { list: PriceList }) {
  const { t } = useTranslation('pricing')
  const formatting = useRecordFormatting()
  if (!list.valid_from && !list.valid_to) {
    return <span className="text-muted-foreground">{t('validity.always')}</span>
  }
  if (list.valid_from && list.valid_to) {
    return (
      <span className="tabular-nums">
        {t('validity.between', {
          from: formatDate(list.valid_from, formatting),
          to: formatDate(list.valid_to, formatting),
        })}
      </span>
    )
  }
  if (list.valid_from) {
    return <span className="tabular-nums">{t('validity.from', { from: formatDate(list.valid_from, formatting) })}</span>
  }
  return <span className="tabular-nums">{t('validity.until', { to: formatDate(list.valid_to, formatting) })}</span>
}
