import { Pencil, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, PageLoader } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { toast } from '@/components/ui/toast-store'
import {PRICE_LISTS_PATH, priceListsApi, useInvalidatePriceLists, usePriceList} from '@/sdk'
import { priceListErrorKey } from './errors'
import { PriceListBadges, PriceListValidity } from './PriceListBadges'
import { PriceListDialog } from './PriceListDialog'
import { PriceListItemsEditor } from './PriceListItemsEditor'
import type { PriceList } from '@/sdk'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'

export function PriceListDetailPage() {
  const { id = '' } = useParams()
  const list = usePriceList(id)

  if (list.isPending) return <PageLoader />
  if (list.isError) return <ErrorState onRetry={() => void list.refetch()} />
  return <PriceListDetail key={list.data.id} list={list.data} />
}

function PriceListDetail({ list }: { list: PriceList }) {
  const { t } = useTranslation('pricing')
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const canEdit = hasPermission('catalog_items.update')
  const invalidate = useInvalidatePriceLists()
  const [editing, setEditing] = useState(false)
  const [removing, setRemoving] = useState(false)

  const remove = async () => {
    try {
      await priceListsApi.remove(list.id)
      await invalidate()
      toast({ title: t('detail.deleted'), variant: 'success' })
      navigate(PRICE_LISTS_PATH)
    } catch (error) {
      toast({ title: t(priceListErrorKey(error)), variant: 'destructive' })
    }
  }

  return (
    <Page>
      <PageHeader
        back={{ to: PRICE_LISTS_PATH, label: t('detail.back') }}
        title={list.name}
        badges={<PriceListBadges list={list} />}
        description={list.description ? <p>{list.description}</p> : undefined}
        actions={
          canEdit && (
            <>
              <Button size="sm" variant="outline" onClick={() => setEditing(true)}>
                <Pencil aria-hidden="true" />
                {t('detail.edit')}
              </Button>
              <Button size="sm" variant="outline" onClick={() => setRemoving(true)}>
                <Trash2 aria-hidden="true" />
                {t('detail.delete')}
              </Button>
            </>
          )
        }
      />
        <dl className="grid gap-3 rounded-lg border bg-card p-4 text-sm sm:grid-cols-3">
          <div className="grid gap-0.5">
            <dt className="text-muted-foreground">{t('columns.currency')}</dt>
            <dd className="font-mono">{list.currency}</dd>
          </div>
          <div className="grid gap-0.5">
            <dt className="text-muted-foreground">{t('columns.appliesTo')}</dt>
            <dd>
              {list.applies_to === 'organizations'
                ? t('detail.organizations', { count: list.company_ids.length })
                : t(`appliesTo.${list.applies_to}`)}
            </dd>
          </div>
          <div className="grid gap-0.5">
            <dt className="text-muted-foreground">{t('columns.validity')}</dt>
            <dd>
              <PriceListValidity list={list} />
            </dd>
          </div>
        </dl>
      <PriceListItemsEditor list={list} canEdit={canEdit} />
      <PriceListDialog open={editing} onOpenChange={setEditing} list={list} />
      <ConfirmDialog
        open={removing}
        onOpenChange={setRemoving}
        title={t('detail.deleteTitle')}
        description={t('detail.deleteDescription', { name: list.name })}
        confirmLabel={t('detail.delete')}
        variant="destructive"
        onConfirm={remove}
      />
    </Page>
  )
}

