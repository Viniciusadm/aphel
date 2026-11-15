import { useQueryClient } from '@tanstack/react-query'
import { Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { api } from '@/api'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { type RecordItem, recordPath, recordsKeys, recordTitle, useNotifyDeleted } from '@/sdk'
import { PRODUCT_RECORD_TYPE, PRODUCTS_PATH, productResource } from './resource'

export interface ProductActionsProps {
  record: RecordItem
}

export function ProductActions({ record }: ProductActionsProps) {
  const { t } = useTranslation('catalog_items')
  const { hasPermission } = useSession()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const notifyDeleted = useNotifyDeleted()
  const [confirming, setConfirming] = useState(false)
  const inactive = record.is_active === false

  const remove = async () => {
    await api.delete<void>(recordPath(productResource, record.id))
    void queryClient.invalidateQueries({ queryKey: recordsKeys.list(PRODUCT_RECORD_TYPE) })
    void queryClient.invalidateQueries({ queryKey: recordsKeys.lookup(PRODUCT_RECORD_TYPE) })
    notifyDeleted({ recordType: PRODUCT_RECORD_TYPE, recordId: record.id, message: t('detail.deleted') })
    navigate(PRODUCTS_PATH)
  }

  return (
    <div className="flex flex-wrap items-center gap-2">
      {inactive && <Badge variant="secondary">{t('detail.inactive')}</Badge>}
      {hasPermission('catalog_items.delete') && (
        <Button size="sm" variant="outline" onClick={() => setConfirming(true)}>
          <Trash2 aria-hidden="true" />
          {t('detail.delete')}
        </Button>
      )}
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={t('detail.deleteTitle')}
        description={t('detail.deleteDescription', { name: recordTitle(productResource, record) })}
        confirmLabel={t('detail.delete')}
        variant="destructive"
        onConfirm={remove}
      />
    </div>
  )
}
