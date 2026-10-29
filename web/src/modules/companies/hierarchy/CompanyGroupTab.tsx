import { Building2, GitBranch, Lock, Plus, Unlink } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { RecordFieldInput } from '@/components/fields'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { formatCurrency, formatNumber, useRecordFormatting, type RecordFormatting } from '@/features/records/formatting'
import { useCompanyGroupTotals, useCompanyHierarchy, useSetCompanyParent } from './api'
import { hierarchyErrorMessage } from './errors'
import { ancestorIds, buildCompanyTree, groupFilterSearch } from './tree'
import type { CompanyGroupTotals, CompanyHierarchyNode, CompanyTreeNode } from './types'

export interface CompanyGroupTabProps {
  companyId: string
  readOnly?: boolean
}

type PickerMode = 'parent' | 'subsidiary'

const COMPANY_RECORD_TYPE = 'company'

function count(value: number | null | undefined, formatting: RecordFormatting): string {
  return value == null ? '—' : formatNumber(value, formatting)
}

function money(
  value: string | null | undefined,
  currency: string | null | undefined,
  formatting: RecordFormatting,
): string {
  return value == null ? '—' : formatCurrency(value, formatting, currency)
}

interface StatProps {
  label: string
  value: string
  hint?: string
  to?: string
}

function Stat({ label, value, hint, to }: StatProps) {
  const body = (
    <>
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd className="text-lg font-semibold tabular-nums">{value}</dd>
      {hint && <dd className="text-xs text-muted-foreground">{hint}</dd>}
    </>
  )
  return (
    <div className="min-w-0 rounded-md border p-3">
      {to ? (
        <Link to={to} className="grid gap-0.5 hover:underline">
          {body}
        </Link>
      ) : (
        <div className="grid gap-0.5">{body}</div>
      )}
    </div>
  )
}

function GroupTotalsPanel({ totals, companyId }: { totals: CompanyGroupTotals; companyId: string }) {
  const { t } = useTranslation('companies', { keyPrefix: 'hierarchy' })
  const formatting = useRecordFormatting()
  const search = groupFilterSearch(companyId)
  return (
    <section aria-labelledby="company-group-totals" className="grid gap-2">
      <h3 id="company-group-totals" className="text-sm font-medium">
        {t('totals.title')}
      </h3>
      <dl className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-4">
        <Stat label={t('totals.companies')} value={count(totals.companies_count, formatting)} />
        <Stat
          label={t('totals.contacts')}
          value={count(totals.contacts_count, formatting)}
          to={totals.contacts_count == null ? undefined : `/contacts?${search}`}
        />
        <Stat
          label={t('totals.openDeals')}
          value={money(totals.open_deals_amount, totals.currency, formatting)}
          hint={t('totals.dealsCount', { count: totals.open_deals_count ?? 0 })}
          to={totals.open_deals_count == null ? undefined : `/deals?${search}`}
        />
        <Stat
          label={t('totals.won')}
          value={money(totals.won_amount, totals.currency, formatting)}
          hint={t('totals.dealsCount', { count: totals.won_deals_count ?? 0 })}
        />
      </dl>
    </section>
  )
}

interface TreeItemProps {
  entry: CompanyTreeNode
  currentId: string
}

function NodeLabel({ node, current }: { node: CompanyHierarchyNode; current: boolean }) {
  const { t } = useTranslation('companies', { keyPrefix: 'hierarchy' })
  if (!node.accessible) {
    return (
      <span className="flex items-center gap-1.5 text-muted-foreground">
        <Lock className="size-3.5" aria-hidden="true" />
        {t('tree.restricted')}
      </span>
    )
  }
  const name = node.name ?? t('tree.unnamed')
  return current ? (
    <span className="truncate font-semibold" aria-current="page">
      {name}
    </span>
  ) : (
    <Link to={`/companies/${encodeURIComponent(node.id)}`} className="truncate font-medium hover:underline">
      {name}
    </Link>
  )
}

function TreeItem({ entry, currentId }: TreeItemProps) {
  const { t } = useTranslation('companies', { keyPrefix: 'hierarchy' })
  const { node, children } = entry
  const current = node.id === currentId
  const details: ReactNode[] = []
  if (node.accessible) {
    if (node.owner) details.push(node.owner.display_name)
    if (node.contacts_count != null) details.push(t('tree.contacts', { count: node.contacts_count }))
    if (node.open_deals_count != null) details.push(t('tree.openDeals', { count: node.open_deals_count }))
  }
  return (
    <li className="grid gap-1">
      <div
        className={`flex min-w-0 flex-wrap items-center justify-between gap-x-3 gap-y-1 rounded-md border px-3 py-2 ${
          current ? 'border-primary bg-primary/5' : ''
        }`}
      >
        <div className="flex min-w-0 items-center gap-2">
          <Building2 className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
          <NodeLabel node={node} current={current} />
          {node.depth === 0 && <Badge variant="secondary">{t('tree.root')}</Badge>}
          {node.archived && <Badge variant="outline">{t('tree.archived')}</Badge>}
        </div>
        {details.length > 0 && <span className="text-xs text-muted-foreground">{details.join(' · ')}</span>}
      </div>
      {children.length > 0 && (
        <ul className="ml-3 grid gap-1 border-l pl-3 sm:ml-5 sm:pl-4">
          {children.map((child) => (
            <TreeItem key={child.node.id} entry={child} currentId={currentId} />
          ))}
        </ul>
      )}
    </li>
  )
}

export function CompanyGroupTab({ companyId, readOnly = false }: CompanyGroupTabProps) {
  const { t } = useTranslation('companies', { keyPrefix: 'hierarchy' })
  const hierarchy = useCompanyHierarchy(companyId)
  const totals = useCompanyGroupTotals(companyId)
  const setParent = useSetCompanyParent()
  const [mode, setMode] = useState<PickerMode | null>(null)
  const [picked, setPicked] = useState<string | null>(null)
  const [detaching, setDetaching] = useState(false)

  if (hierarchy.isPending) {
    return (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-20 w-full" />
        <Skeleton className="h-12 w-full" />
        <Skeleton className="h-12 w-full" />
      </div>
    )
  }
  if (hierarchy.isError) {
    return <ErrorState title={t('loadError')} onRetry={() => void hierarchy.refetch()} />
  }

  const data = hierarchy.data
  const current = data.nodes.find((node) => node.id === companyId)
  const parentId = current?.parent_id ?? null
  const tree = buildCompanyTree(data.nodes)
  const blocked = ancestorIds(data.nodes, companyId)
  const alone = data.nodes.length <= 1

  const fail = (error: unknown) => {
    toast({ title: t('errors.title'), description: hierarchyErrorMessage(error, t), variant: 'destructive' })
  }

  const openPicker = (next: PickerMode) => {
    setPicked(null)
    setMode(next)
  }

  const confirmPick = async () => {
    if (!picked || !mode) return
    if (mode === 'subsidiary' && (picked === companyId || blocked.has(picked))) {
      const message = t('errors.cycle')
      toast({ title: t('errors.title'), description: message, variant: 'destructive' })
      throw new Error(message)
    }
    try {
      if (mode === 'parent') {
        await setParent.mutateAsync({ companyId, parentId: picked })
        notifySuccess(t('parent.saved'))
      } else {
        await setParent.mutateAsync({ companyId: picked, parentId: companyId })
        notifySuccess(t('subsidiary.saved'))
      }
    } catch (error) {
      fail(error)
      throw error
    }
  }

  const detach = async () => {
    try {
      await setParent.mutateAsync({ companyId, parentId: null })
      notifySuccess(t('parent.removed'))
    } catch (error) {
      fail(error)
      throw error
    }
  }

  return (
    <div className="grid gap-4">
      {totals.isError ? (
        <ErrorState title={t('totals.loadError')} onRetry={() => void totals.refetch()} />
      ) : totals.data ? (
        <GroupTotalsPanel totals={totals.data} companyId={companyId} />
      ) : (
        <Skeleton className="h-20 w-full" />
      )}

      <section aria-labelledby="company-group-tree" className="grid gap-2">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h3 id="company-group-tree" className="text-sm font-medium">
            {t('tree.title')}
          </h3>
          {!readOnly && (
            <div className="flex flex-wrap gap-2">
              <Button size="sm" variant="outline" onClick={() => openPicker('parent')}>
                <GitBranch aria-hidden="true" />
                {parentId ? t('parent.change') : t('parent.set')}
              </Button>
              {parentId && (
                <Button size="sm" variant="outline" onClick={() => setDetaching(true)}>
                  <Unlink aria-hidden="true" />
                  {t('parent.remove')}
                </Button>
              )}
              <Button size="sm" variant="outline" onClick={() => openPicker('subsidiary')}>
                <Plus aria-hidden="true" />
                {t('subsidiary.add')}
              </Button>
            </div>
          )}
        </div>
        {alone && <p className="text-sm text-muted-foreground">{t('tree.alone')}</p>}
        <ul aria-label={t('tree.title')} className="grid gap-1">
          {tree.map((entry) => (
            <TreeItem key={entry.node.id} entry={entry} currentId={companyId} />
          ))}
        </ul>
        {data.truncated && <p className="text-xs text-muted-foreground">{t('tree.truncated')}</p>}
        <p className="text-xs text-muted-foreground">{t('tree.maxDepth', { count: data.max_depth })}</p>
      </section>

      <ConfirmDialog
        open={mode !== null}
        onOpenChange={(open) => !open && setMode(null)}
        title={mode === 'subsidiary' ? t('subsidiary.title') : t('parent.title')}
        description={mode === 'subsidiary' ? t('subsidiary.description') : t('parent.description')}
        confirmLabel={t('save')}
        notifyOnError={false}
        pending={setParent.isPending}
        onConfirm={confirmPick}
      >
        <RecordFieldInput
          targetRecordType={COMPANY_RECORD_TYPE}
          value={picked}
          onChange={setPicked}
          aria-label={mode === 'subsidiary' ? t('subsidiary.picker') : t('parent.picker')}
        />
      </ConfirmDialog>

      <ConfirmDialog
        open={detaching}
        onOpenChange={setDetaching}
        title={t('parent.removeTitle')}
        description={t('parent.removeDescription')}
        confirmLabel={t('parent.remove')}
        notifyOnError={false}
        pending={setParent.isPending}
        onConfirm={detach}
      />
    </div>
  )
}
