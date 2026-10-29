import { ArrowRight, Building2, Loader2, Search } from 'lucide-react'
import { useMemo, useState, type FormEvent } from 'react'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { useFieldSchema } from '@/features/records/api'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import type { RecordTabRenderProps } from '@/kernel/modules/define'
import {
  ENRICHMENT_KEY,
  USE_PERMISSION,
  enrichmentApi,
  useEnrichmentCompany,
  useInvalidateEnrichment,
  type EnrichmentLookup,
  type EnrichmentProposalItem,
} from './api'
import { normalizeCnpj } from './cnpj'

const KNOWN_ERRORS = [
  'enrichment_cnpj_missing',
  'enrichment_invalid_cnpj',
  'enrichment_cnpj_not_found',
  'enrichment_source_rate_limited',
  'enrichment_source_unavailable',
  'enrichment_lookup_expired',
  'enrichment_lookup_not_found',
  'enrichment_nothing_to_apply',
  'company_not_found',
  'rate_limited',
  'permission_denied',
]

function notifyKnown(error: unknown, translate: (key: string) => string) {
  if (isApiError(error) && KNOWN_ERRORS.includes(error.code)) {
    notifyError(error, { title: translate(`errors.${error.code}`) })
  } else {
    notifyError(error)
  }
}

function displayValue(value: unknown): string {
  if (value === null || value === undefined) return ''
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  return JSON.stringify(value)
}

function selectable(item: EnrichmentProposalItem): boolean {
  return Boolean(item.field_key) && item.changed
}

export function CompanyEnrichmentTab({ recordId, readOnly }: RecordTabRenderProps) {
  const t = useModuleTranslation(ENRICHMENT_KEY)
  const { hasPermission } = useSession()
  const formatting = useRecordFormatting()
  const overview = useEnrichmentCompany(recordId)
  const schema = useFieldSchema('company')
  const invalidate = useInvalidateEnrichment()
  const [typedCnpj, setTypedCnpj] = useState<string | null>(null)
  const [lookup, setLookup] = useState<EnrichmentLookup | null>(null)
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [searching, setSearching] = useState(false)
  const [applying, setApplying] = useState(false)
  const canLookup = !readOnly && hasPermission(USE_PERMISSION)
  const canApply = canLookup && hasPermission('companies.update')

  const cnpj = typedCnpj ?? overview.data?.cnpj ?? ''

  const fieldLabels = useMemo(() => {
    const labels = new Map<string, string>()
    for (const field of schema.data?.fields ?? []) labels.set(field.key, field.label)
    return labels
  }, [schema.data])

  const valid = cnpj.trim() === '' ? Boolean(overview.data?.cnpj_field) : normalizeCnpj(cnpj) !== null

  const search = async (event: FormEvent) => {
    event.preventDefault()
    if (!canLookup || !valid) return
    setSearching(true)
    try {
      const result = await enrichmentApi.lookup(recordId, cnpj.trim() || undefined)
      setLookup(result)
      setSelected(new Set(result.items.filter(selectable).map((item) => item.data_key)))
    } catch (error) {
      notifyKnown(error, t)
    } finally {
      setSearching(false)
    }
  }

  const toggle = (key: string, checked: boolean) =>
    setSelected((current) => {
      const next = new Set(current)
      if (checked) next.add(key)
      else next.delete(key)
      return next
    })

  const apply = async () => {
    if (!lookup || selected.size === 0) return
    setApplying(true)
    try {
      const result = await enrichmentApi.apply(lookup.id, [...selected])
      notifySuccess(t('applied', { count: result.fields.length }))
      setLookup(null)
      setSelected(new Set())
      await invalidate()
    } catch (error) {
      notifyKnown(error, t)
    } finally {
      setApplying(false)
    }
  }

  return (
    <section className="flex flex-col gap-4" aria-label={t('tab')}>
      <div className="grid gap-0.5">
        <h2 className="text-base font-semibold">{t('company.title')}</h2>
        <p className="text-sm text-muted-foreground">{t('company.description')}</p>
      </div>
      {canLookup && (
        <form className="grid gap-1.5" onSubmit={(event) => void search(event)}>
          <Label htmlFor="enrichment-cnpj">{t('company.cnpj')}</Label>
          <div className="flex flex-col gap-2 sm:flex-row">
            <Input
              id="enrichment-cnpj"
              inputMode="text"
              autoComplete="off"
              maxLength={18}
              placeholder="00.000.000/0000-00"
              value={cnpj}
              aria-invalid={cnpj.trim() !== '' && !valid}
              disabled={searching}
              onChange={(event) => setTypedCnpj(event.target.value)}
              className="sm:max-w-xs"
            />
            <Button type="submit" disabled={!valid || searching}>
              {searching ? <Loader2 className="animate-spin" aria-hidden="true" /> : <Search aria-hidden="true" />}
              {searching ? t('company.searching') : t('company.search')}
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            {cnpj.trim() !== '' && !valid ? t('company.invalidCnpj') : t('company.dataNotice')}
          </p>
        </form>
      )}
      {lookup && (
        <div className="grid gap-3 rounded-md border bg-card p-3">
          <div className="flex flex-wrap items-center gap-2">
            <p className="text-sm font-medium">{t('lookup.title', { cnpj: lookup.cnpj })}</p>
            <Badge variant="secondary">{t(`sources.${lookup.source}`)}</Badge>
            <span className="text-xs text-muted-foreground">{formatDateTime(lookup.fetched_at, formatting)}</span>
          </div>
          <ul className="grid gap-2">
            {lookup.items.map((item) => {
              const id = `enrichment-item-${item.data_key}`
              const enabled = canApply && selectable(item)
              return (
                <li key={item.data_key} className="flex items-start gap-3 rounded-md border px-3 py-2">
                  <Checkbox
                    id={id}
                    className="mt-0.5"
                    checked={selected.has(item.data_key)}
                    disabled={!enabled || applying}
                    onCheckedChange={(checked) => toggle(item.data_key, checked === true)}
                  />
                  <div className="grid min-w-0 flex-1 gap-0.5">
                    <Label htmlFor={id} className="text-sm font-medium">
                      {t(`items.${item.data_key}`)}
                      <span className="ml-1 font-normal text-muted-foreground">
                        {item.field_key
                          ? t('lookup.target', { field: item.field_label ?? item.field_key })
                          : t('lookup.unmapped')}
                      </span>
                    </Label>
                    <div className="flex min-w-0 flex-wrap items-center gap-1 text-sm">
                      {item.field_key && displayValue(item.current) !== '' && (
                        <>
                          <span className="min-w-0 break-words text-muted-foreground line-through">
                            {displayValue(item.current)}
                          </span>
                          <ArrowRight className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
                        </>
                      )}
                      <span className="min-w-0 break-words">{item.proposed}</span>
                      {item.field_key && !item.changed && (
                        <Badge variant="outline" className="ml-1">
                          {t('lookup.same')}
                        </Badge>
                      )}
                    </div>
                  </div>
                </li>
              )
            })}
          </ul>
          <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
            <Button variant="outline" disabled={applying} onClick={() => setLookup(null)}>
              {t('lookup.discard')}
            </Button>
            {canApply && (
              <Button disabled={selected.size === 0 || applying} onClick={() => void apply()}>
                {applying && <Loader2 className="animate-spin" aria-hidden="true" />}
                {t('lookup.apply', { count: selected.size })}
              </Button>
            )}
          </div>
        </div>
      )}
      <div className="grid gap-2">
        <h3 className="text-sm font-semibold">{t('history.title')}</h3>
        {overview.isPending ? (
          <Skeleton className="h-16 w-full" />
        ) : overview.isError ? (
          <ErrorState onRetry={() => void overview.refetch()} />
        ) : overview.data.sources.length === 0 ? (
          <EmptyState icon={Building2} title={t('history.emptyTitle')} description={t('history.emptyDescription')} />
        ) : (
          <ul className="grid gap-1.5">
            {overview.data.sources.map((source) => (
              <li
                key={source.field_key}
                className="grid gap-0.5 rounded-md border px-3 py-2 text-sm sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center sm:gap-3"
              >
                <div className="grid min-w-0 gap-0.5">
                  <span className="font-medium">{fieldLabels.get(source.field_key) ?? source.field_key}</span>
                  {source.value_snapshot && (
                    <span className="truncate text-muted-foreground">{source.value_snapshot}</span>
                  )}
                </div>
                <span className="text-xs text-muted-foreground">
                  {t('history.source', {
                    source: t(`sources.${source.source}`),
                    cnpj: source.cnpj,
                    date: formatDateTime(source.applied_at, formatting),
                  })}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  )
}
