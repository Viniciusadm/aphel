import { useQuery } from '@tanstack/react-query'
import { useId, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '@/api'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field } from '@/components/ui/field'
import { NativeSelect } from '@/components/ui/native-select'
import { SearchInput } from '@/components/ui/search-input'
import { RequestApprovalDialog, entityLabel, useApprovalPolicies, useEntities, type RecordItem, type RecordsPage } from '@/sdk'
import { cn } from '@/lib/utils'

const RESULTS_LIMIT = 8
const ON_DEMAND = 'on_demand'

interface PickedRecord {
  id: string
  title: string
}

export function NewRequestDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>{open && <NewRequestBody onOpenChange={onOpenChange} />}</DialogContent>
    </Dialog>
  )
}

function NewRequestBody({ onOpenChange }: { onOpenChange: (open: boolean) => void }) {
  const { t, i18n } = useTranslation('approvals')
  const typeId = useId()
  const searchId = useId()
  const policies = useApprovalPolicies()
  const entities = useEntities()
  const [recordType, setRecordType] = useState('')
  const [query, setQuery] = useState('')
  const [picked, setPicked] = useState<PickedRecord | null>(null)
  const [confirming, setConfirming] = useState(false)

  const types = useMemo(() => {
    const keys = new Set(
      (policies.data ?? []).filter((policy) => policy.is_active && policy.moment === ON_DEMAND).map((policy) => policy.record_type),
    )
    return [...keys]
      .flatMap((key) => {
        const entity = entities.data?.find((item) => item.record_type === key)
        return entity ? [{ key, label: entityLabel(entity, i18n.language, key) }] : []
      })
      .sort((left, right) => left.label.localeCompare(right.label))
  }, [policies.data, entities.data, i18n.language])

  const selectedType = recordType || types[0]?.key || ''
  const titleField = entities.data?.find((entity) => entity.record_type === selectedType)?.title_field
  const search = query.trim()

  const records = useQuery({
    queryKey: ['approvals', 'new-request', 'records', selectedType, search],
    enabled: selectedType !== '' && !picked,
    queryFn: async () =>
      (
        await api.get<RecordsPage<RecordItem>>(`/v1/entities/${encodeURIComponent(selectedType)}/records`, {
          query: { per_page: RESULTS_LIMIT, ...(search ? { search } : {}) },
        })
      ).data,
  })

  const titleOf = (record: RecordItem) => {
    const value = titleField ? record[titleField] : undefined
    return typeof value === 'string' && value !== '' ? value : record.id
  }

  if (confirming && picked) {
    return (
      <RequestApprovalDialog
        open
        onOpenChange={(next) => {
          if (!next) onOpenChange(false)
        }}
        recordType={selectedType}
        recordId={picked.id}
        moment={ON_DEMAND}
      />
    )
  }

  let list
  if (policies.isPending || entities.isPending) {
    list = null
  } else if (types.length === 0) {
    list = <p className="text-sm text-muted-foreground">{t('newRequest.noPolicies')}</p>
  } else if (picked) {
    list = (
      <div className="flex min-h-11 items-center gap-2 rounded-md border bg-muted/40 px-3">
        <span className="min-w-0 flex-1 truncate text-sm">{picked.title}</span>
        <Button type="button" variant="ghost" size="sm" onClick={() => setPicked(null)}>
          {t('newRequest.change')}
        </Button>
      </div>
    )
  } else {
    const items = records.data ?? []
    list = (
      <>
        <SearchInput
          id={searchId}
          value={query}
          onValueChange={setQuery}
          placeholder={t('newRequest.searchPlaceholder')}
          autoComplete="off"
        />
        <ul className="grid divide-y rounded-md border" aria-label={t('newRequest.results')} aria-busy={records.isFetching}>
          {items.length === 0 ? (
            <li className="px-3 py-3 text-sm text-muted-foreground">
              {records.isFetching ? t('newRequest.searching') : t('newRequest.empty')}
            </li>
          ) : (
            items.map((record) => (
              <li key={record.id}>
                <button
                  type="button"
                  className={cn('flex min-h-11 w-full items-center px-3 py-2 text-left text-sm hover:bg-accent focus-visible:bg-accent focus-visible:outline-none')}
                  onClick={() => setPicked({ id: record.id, title: titleOf(record) })}
                >
                  <span className="truncate">{titleOf(record)}</span>
                </button>
              </li>
            ))
          )}
        </ul>
      </>
    )
  }

  return (
    <div className="grid gap-4">
      <DialogHeader>
        <DialogTitle>{t('newRequest.title')}</DialogTitle>
        <DialogDescription>{t('newRequest.description')}</DialogDescription>
      </DialogHeader>
      {types.length > 0 && (
        <Field id={typeId} label={t('newRequest.recordType')}>
          <NativeSelect
            id={typeId}
            value={selectedType}
            onChange={(event) => {
              setRecordType(event.target.value)
              setPicked(null)
              setQuery('')
            }}
          >
            {types.map((type) => (
              <option key={type.key} value={type.key}>
                {type.label}
              </option>
            ))}
          </NativeSelect>
        </Field>
      )}
      {list && (
        <div className="grid gap-2">
          {types.length > 0 && <span className="text-sm font-medium">{t('newRequest.record')}</span>}
          {list}
        </div>
      )}
      <DialogFooter>
        <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
          {t('actions.close')}
        </Button>
        <Button type="button" disabled={!picked} onClick={() => setConfirming(true)}>
          {t('newRequest.continue')}
        </Button>
      </DialogFooter>
    </div>
  )
}
