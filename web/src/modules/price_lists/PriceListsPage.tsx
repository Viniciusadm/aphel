import { Plus, Tags } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { PageLoader } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { SectionWorkspace, SectionContent, SectionIndex, SectionLink } from '@/components/ui/section-workspace'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useCurrencySettings } from '@/modules/money/api'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { PRICE_LISTS_PER_PAGE, priceListPath, usePriceLists } from './api'
import { DiscountLimitsPanel } from './DiscountLimitsPanel'
import { PriceListBadges, PriceListValidity } from './PriceListBadges'
import { PriceListDialog } from './PriceListDialog'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Page } from '@/components/ui/page'
import { SearchInput } from '@/components/ui/search-input'

const PRODUCTS_PATH = '/products'

export function PriceListsPage() {
  const { t } = useTranslation('price_lists')
  const { hasPermission } = useSession()
  const canManageLimits = hasPermission(SETTINGS_MANAGE)

  return (
    <Page>
      <PageHeader
        back={{ to: PRODUCTS_PATH, label: t('list.back') }}
        title={t('list.title')}
        description={t('list.description')}
      />
      {canManageLimits ? (
        <SectionWorkspace defaultValue="lists">
          <SectionIndex>
            <SectionLink value="lists">{t('tabs.lists')}</SectionLink>
            <SectionLink value="limits">{t('tabs.limits')}</SectionLink>
          </SectionIndex>
          <SectionContent value="lists">
            <PriceListsTable />
          </SectionContent>
          <SectionContent value="limits">
            <DiscountLimitsPanel />
          </SectionContent>
        </SectionWorkspace>
      ) : (
        <PriceListsTable />
      )}
    </Page>
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
        <SearchInput
          wrapperClassName="w-full sm:w-64"
          value={search}
          placeholder={t('list.search')}
          aria-label={t('list.search')}
          onValueChange={(value) => {
            setSearch(value)
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
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">
                  {t('columns.name')}
                </TableHead>
                <TableHead scope="col">
                  {t('columns.currency')}
                </TableHead>
                <TableHead scope="col" className="hidden md:table-cell">
                  {t('columns.appliesTo')}
                </TableHead>
                <TableHead scope="col" className="hidden md:table-cell">
                  {t('columns.validity')}
                </TableHead>
                <TableHead scope="col" className="text-right">
                  {t('columns.items')}
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {lists.data.data.map((list) => (
                <TableRow
                  key={list.id}
                  className="cursor-pointer hover:bg-muted/40"
                  onClick={() => navigate(priceListPath(list.id))}
                >
                  <TableCell>
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
                  </TableCell>
                  <TableCell className="font-mono">{list.currency}</TableCell>
                  <TableCell className="hidden md:table-cell">{t(`appliesTo.${list.applies_to}`)}</TableCell>
                  <TableCell className="hidden md:table-cell">
                    <PriceListValidity list={list} />
                  </TableCell>
                  <TableCell className="text-right tabular-nums">{list.item_count}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
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
