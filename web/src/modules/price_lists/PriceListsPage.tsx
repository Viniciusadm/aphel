import { ArrowLeft, Plus, Tags } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { PageLoader } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useCurrencySettings } from '@/modules/currencies/api'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { PRICE_LISTS_PER_PAGE, priceListPath, usePriceLists } from './api'
import { DiscountLimitsPanel } from './DiscountLimitsPanel'
import { PriceListBadges, PriceListValidity } from './PriceListBadges'
import { PriceListDialog } from './PriceListDialog'

const PRODUCTS_PATH = '/products'

export function PriceListsPage() {
  const { t } = useTranslation('price_lists')
  const { hasPermission } = useSession()
  const canManageLimits = hasPermission(SETTINGS_MANAGE)

  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-2">
        <Button asChild size="sm" variant="ghost" className="w-fit">
          <Link to={PRODUCTS_PATH}>
            <ArrowLeft aria-hidden="true" />
            {t('list.back')}
          </Link>
        </Button>
        <div className="grid gap-1">
          <h1 className="text-xl font-semibold">{t('list.title')}</h1>
          <p className="text-sm text-muted-foreground">{t('list.description')}</p>
        </div>
      </div>
      {canManageLimits ? (
        <Tabs defaultValue="lists" className="flex flex-col gap-4">
          <TabsList>
            <TabsTrigger value="lists">{t('tabs.lists')}</TabsTrigger>
            <TabsTrigger value="limits">{t('tabs.limits')}</TabsTrigger>
          </TabsList>
          <TabsContent value="lists">
            <PriceListsTable />
          </TabsContent>
          <TabsContent value="limits">
            <DiscountLimitsPanel />
          </TabsContent>
        </Tabs>
      ) : (
        <PriceListsTable />
      )}
    </div>
  )
}

function PriceListsTable() {
  const { t } = useTranslation('price_lists')
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const canEdit = hasPermission('products.update')
  const [page, setPage] = useState(1)
  const [search, setSearch] = useState('')
  const [currency, setCurrency] = useState('')
  const [creating, setCreating] = useState(false)
  const currencies = useCurrencySettings()
  const lists = usePriceLists({ page, search, currency })
  const lastPage = lists.data ? Math.max(1, Math.ceil(lists.data.meta.total / PRICE_LISTS_PER_PAGE)) : 1
  const codes = currencies.data?.currencies.map((item) => item.code) ?? []
  const newButton = canEdit ? (
    <Button size="sm" onClick={() => setCreating(true)}>
      <Plus aria-hidden="true" />
      {t('list.new')}
    </Button>
  ) : undefined

  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-center gap-2">
        <Input
          className="w-full sm:w-64"
          type="search"
          value={search}
          placeholder={t('list.search')}
          aria-label={t('list.search')}
          onChange={(event) => {
            setSearch(event.target.value)
            setPage(1)
          }}
        />
        <NativeSelect
          aria-label={t('list.currencyFilter')}
          value={currency}
          onChange={(event) => {
            setCurrency(event.target.value)
            setPage(1)
          }}
        >
          <option value="">{t('list.allCurrencies')}</option>
          {codes.map((code) => (
            <option key={code} value={code}>
              {code}
            </option>
          ))}
        </NativeSelect>
        <div className="ml-auto">{newButton}</div>
      </div>
      {lists.isPending ? (
        <PageLoader />
      ) : lists.isError ? (
        <ErrorState onRetry={() => void lists.refetch()} />
      ) : lists.data.data.length === 0 ? (
        <EmptyState icon={Tags} title={t('list.empty')} description={t('list.emptyHint')} action={newButton} />
      ) : (
        <>
          <div className="relative overflow-x-auto rounded-lg border">
            <table className="w-full text-sm">
              <thead className="bg-muted/50 text-left text-muted-foreground">
                <tr>
                  <th scope="col" className="px-4 py-2 font-medium">
                    {t('columns.name')}
                  </th>
                  <th scope="col" className="px-4 py-2 font-medium">
                    {t('columns.currency')}
                  </th>
                  <th scope="col" className="hidden px-4 py-2 font-medium md:table-cell">
                    {t('columns.appliesTo')}
                  </th>
                  <th scope="col" className="hidden px-4 py-2 font-medium md:table-cell">
                    {t('columns.validity')}
                  </th>
                  <th scope="col" className="px-4 py-2 text-right font-medium">
                    {t('columns.items')}
                  </th>
                </tr>
              </thead>
              <tbody>
                {lists.data.data.map((list) => (
                  <tr
                    key={list.id}
                    className="cursor-pointer border-t hover:bg-muted/40"
                    onClick={() => navigate(priceListPath(list.id))}
                  >
                    <td className="px-4 py-2">
                      <div className="flex flex-wrap items-center gap-2">
                        <Link
                          to={priceListPath(list.id)}
                          className="font-medium hover:underline"
                          onClick={(event) => event.stopPropagation()}
                        >
                          {list.name}
                        </Link>
                        <PriceListBadges list={list} />
                      </div>
                    </td>
                    <td className="px-4 py-2 font-mono">{list.currency}</td>
                    <td className="hidden px-4 py-2 md:table-cell">{t(`appliesTo.${list.applies_to}`)}</td>
                    <td className="hidden px-4 py-2 md:table-cell">
                      <PriceListValidity list={list} />
                    </td>
                    <td className="px-4 py-2 text-right tabular-nums">{list.item_count}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {lastPage > 1 && (
            <div className="flex items-center justify-end gap-2 text-sm">
              <Button size="sm" variant="outline" disabled={page <= 1} onClick={() => setPage(page - 1)}>
                {t('list.previous')}
              </Button>
              <span className="tabular-nums text-muted-foreground">{t('list.page', { page, total: lastPage })}</span>
              <Button size="sm" variant="outline" disabled={page >= lastPage} onClick={() => setPage(page + 1)}>
                {t('list.next')}
              </Button>
            </div>
          )}
        </>
      )}
      {canEdit && (
        <PriceListDialog
          open={creating}
          onOpenChange={setCreating}
          list={null}
          onSaved={(list) => navigate(priceListPath(list.id))}
        />
      )}
    </div>
  )
}
