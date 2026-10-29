import { ArrowLeft, ArrowLeftRight, GitMerge, SearchX } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { PageLoader } from '@/components/feedback/PageLoader'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { useFieldSchema } from '@/features/records/api'
import { useRecord } from '@/features/records/detail'
import { FieldValueDisplay } from '@/features/records/display/FieldValueDisplay'
import { isEmptyValue } from '@/features/records/display/values'
import { getRecordResource, recordTitle, type RecordResource } from '@/features/records/resources'
import type { FieldSchema, RecordItem } from '@/features/records/types'
import '@/modules/companies/resource'
import '@/modules/contacts/resource'
import '@/modules/products/resource'
import { cn } from '@/lib/utils'
import { useDuplicateTypes, useMergeRecords, type DuplicateTypeInfo } from './api'
import { defaultChoices, mergeableFields, mergeRows, movedTotal, type MergeRow, type MergeSide } from './lib/merge'
import { DUPLICATES_SETTINGS_PATH } from './paths'

export function MergePage() {
  const { t } = useTranslation('duplicates')
  const [searchParams] = useSearchParams()
  const types = useDuplicateTypes()
  const type = searchParams.get('type')
  const primary = searchParams.get('primary') ?? ''
  const duplicate = searchParams.get('duplicate') ?? ''

  if (types.isPending) return <PageLoader label={t('merge.title')} />
  if (types.isError) return <ErrorState onRetry={() => void types.refetch()} />
  const info = types.data.find((item) => item.record_type === type)
  const resource = type ? getRecordResource(type) : undefined
  if (!info || !resource || !primary || !duplicate || primary === duplicate) {
    return <MergeUnavailable message={t('merge.invalid')} />
  }
  return (
    <MergeLoader
      key={`${primary}:${duplicate}`}
      info={info}
      resource={resource}
      primaryId={primary}
      duplicateId={duplicate}
    />
  )
}

function MergeUnavailable({ message }: { message: string }) {
  const { t } = useTranslation('duplicates')
  return (
    <EmptyState
      icon={SearchX}
      title={message}
      action={
        <Button variant="outline" asChild>
          <Link to={DUPLICATES_SETTINGS_PATH}>{t('merge.back')}</Link>
        </Button>
      }
    />
  )
}

interface MergeLoaderProps {
  info: DuplicateTypeInfo
  resource: RecordResource
  primaryId: string
  duplicateId: string
}

function MergeLoader({ info, resource, primaryId, duplicateId }: MergeLoaderProps) {
  const { t } = useTranslation('duplicates')
  const recordType = info.record_type
  const [order, setOrder] = useState<[string, string]>([primaryId, duplicateId])
  const schema = useFieldSchema(recordType)
  const first = useRecord(recordType, primaryId)
  const second = useRecord(recordType, duplicateId)

  if ([first, second].some((query) => query.isError && isApiError(query.error) && query.error.status === 404)) {
    return <MergeUnavailable message={t('merge.missing')} />
  }
  if (first.isError || second.isError || schema.isError) {
    return (
      <ErrorState
        onRetry={() => {
          void first.refetch()
          void second.refetch()
          void schema.refetch()
        }}
      />
    )
  }
  if (!first.data || !second.data || !schema.data) return <PageLoader label={t('merge.title')} />
  if (first.data.id !== primaryId || second.data.id !== duplicateId) {
    return <MergeUnavailable message={t('merge.missing')} />
  }

  const records: Record<string, RecordItem> = { [primaryId]: first.data, [duplicateId]: second.data }
  const [primaryKey, duplicateKey] = order
  return (
    <MergeForm
      key={order.join(':')}
      info={info}
      resource={resource}
      schema={schema.data}
      primary={records[primaryKey]!}
      duplicate={records[duplicateKey]!}
      onSwap={() => setOrder(([left, right]) => [right, left])}
    />
  )
}

interface MergeFormProps {
  info: DuplicateTypeInfo
  resource: RecordResource
  schema: FieldSchema
  primary: RecordItem
  duplicate: RecordItem
  onSwap: () => void
}

function MergeForm({ info, resource, schema, primary, duplicate, onSwap }: MergeFormProps) {
  const { t } = useTranslation('duplicates')
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const merge = useMergeRecords()
  const recordType = info.record_type
  const rows = useMemo(
    () => mergeRows(mergeableFields(schema, info.merge_fields), primary, duplicate),
    [schema, info.merge_fields, primary, duplicate],
  )
  const [choices, setChoices] = useState<Record<string, MergeSide>>(() => defaultChoices(rows))
  const [confirming, setConfirming] = useState(false)
  const permissions = info.permissions
  const allowed = hasPermission(permissions.update) && hasPermission(permissions.delete)
  const primaryName = recordTitle(resource, primary)
  const duplicateName = recordTitle(resource, duplicate)

  const submit = async () => {
    const result = await merge.mutateAsync({
      record_type: recordType,
      primary_id: primary.id,
      duplicate_id: duplicate.id,
      fields: choices,
      version: typeof primary.updated_at === 'string' ? primary.updated_at : null,
    })
    notifySuccess(t('merge.success', { count: movedTotal(result.moved) }))
    navigate(resource.detailPath?.(result.survivor_id) ?? DUPLICATES_SETTINGS_PATH)
  }

  return (
    <section className="grid gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="grid gap-1">
          <Link
            to={DUPLICATES_SETTINGS_PATH}
            className="inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
          >
            <ArrowLeft aria-hidden="true" className="size-4" />
            {t('merge.back')}
          </Link>
          <h2 className="text-lg font-semibold">{t('merge.title')}</h2>
          <p className="max-w-3xl text-sm text-muted-foreground">{t('merge.description')}</p>
        </div>
        <Button size="sm" variant="outline" onClick={onSwap}>
          <ArrowLeftRight aria-hidden="true" />
          {t('merge.swap')}
        </Button>
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <SideHeader label={t('merge.primary')} name={primaryName} highlighted />
        <SideHeader label={t('merge.duplicate')} name={duplicateName} />
      </div>
      <div className="grid gap-2">
        {rows.map((row) => (
          <MergeFieldRow
            key={row.field.key}
            row={row}
            choice={choices[row.field.key]}
            onChoose={(side) => setChoices((previous) => ({ ...previous, [row.field.key]: side }))}
          />
        ))}
      </div>
      {!allowed && <p className="text-sm text-destructive">{t('merge.noPermission')}</p>}
      <div className="flex justify-end">
        <Button disabled={!allowed || merge.isPending} onClick={() => setConfirming(true)}>
          <GitMerge aria-hidden="true" />
          {t('merge.submit')}
        </Button>
      </div>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={t('merge.confirmTitle', { primary: primaryName, duplicate: duplicateName })}
        description={t('merge.confirmDescription')}
        confirmLabel={t('merge.submit')}
        pending={merge.isPending}
        onConfirm={submit}
      />
    </section>
  )
}

function SideHeader({ label, name, highlighted = false }: { label: string; name: string; highlighted?: boolean }) {
  return (
    <div className={cn('grid gap-0.5 rounded-lg border p-3', highlighted && 'border-primary')}>
      <span className="text-xs font-medium tracking-wide text-muted-foreground uppercase">{label}</span>
      <span className="truncate font-medium">{name}</span>
    </div>
  )
}

interface MergeFieldRowProps {
  row: MergeRow
  choice: MergeSide | undefined
  onChoose: (side: MergeSide) => void
}

function MergeFieldRow({ row, choice, onChoose }: MergeFieldRowProps) {
  const { t } = useTranslation('duplicates')
  const label = row.field.label
  if (row.same) {
    return (
      <div className="grid gap-1 rounded-lg border border-dashed px-3 py-2 sm:grid-cols-[12rem_1fr] sm:items-center">
        <span className="text-sm font-medium">{label}</span>
        <div className="flex min-w-0 items-center justify-between gap-2 text-sm">
          <ValueOrEmpty row={row} side="primary" />
          <span className="shrink-0 text-xs text-muted-foreground">{t('merge.same')}</span>
        </div>
      </div>
    )
  }
  return (
    <fieldset className="grid gap-2 rounded-lg border px-3 py-2 sm:grid-cols-[12rem_1fr_1fr] sm:items-center">
      <legend className="sr-only">{label}</legend>
      <span aria-hidden="true" className="text-sm font-medium">
        {label}
      </span>
      {(['primary', 'duplicate'] as const).map((side) => (
        <label
          key={side}
          className={cn(
            'flex min-w-0 cursor-pointer items-center gap-2 rounded-md border px-2 py-1.5 text-sm',
            choice === side ? 'border-primary bg-primary/5' : 'border-transparent',
          )}
        >
          <input
            type="radio"
            name={`merge-${row.field.key}`}
            value={side}
            checked={choice === side}
            onChange={() => onChoose(side)}
            aria-label={t(side === 'primary' ? 'merge.keepPrimary' : 'merge.keepDuplicate', { field: label })}
            className="size-4 shrink-0 accent-primary"
          />
          <ValueOrEmpty row={row} side={side} />
        </label>
      ))}
    </fieldset>
  )
}

function ValueOrEmpty({ row, side }: { row: MergeRow; side: MergeSide }) {
  const { t } = useTranslation('duplicates')
  const value = side === 'primary' ? row.primary : row.duplicate
  if (isEmptyValue(value)) return <span className="text-muted-foreground italic">{t('merge.empty')}</span>
  return <FieldValueDisplay field={row.field} value={value} className="min-w-0 break-words" />
}
