import { Download, FileDown, LoaderCircle } from 'lucide-react'
import { useCustomObjectLabel } from '@/features/custom-records/api'
import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { errorMessage } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { fileDownloadUrl } from '@/features/files'
import { sortToParam } from '@/features/records/table'
import type { ArchivedMode } from '@/features/records/types'
import { isExportable, useExportJob, useRequestExport, type ExportFormat } from './api'
import { orderedColumns, useCurrentListConfig } from './exportColumns'

const FORMATS: ExportFormat[] = ['csv', 'xlsx']
const ID_COLUMN = 'id'

export interface ExportDialogProps {
  recordType: string
  open: boolean
  onOpenChange: (open: boolean) => void
  archived?: ArchivedMode
}

export function ExportDialog({ recordType, open, onOpenChange, archived }: ExportDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      {open && <ExportDialogContent recordType={recordType} archived={archived} onClose={() => onOpenChange(false)} />}
    </Dialog>
  )
}

interface ExportDialogContentProps {
  recordType: string
  archived?: ArchivedMode
  onClose: () => void
}

function ExportDialogContent({ recordType, archived, onClose }: ExportDialogContentProps) {
  const { t } = useTranslation('exports')
  const { config, fields, search } = useCurrentListConfig(recordType)
  const [format, setFormat] = useState<ExportFormat>('csv')
  const [selected, setSelected] = useState<Set<string> | null>(null)
  const [includeId, setIncludeId] = useState(false)
  const [exportId, setExportId] = useState<string | null>(null)
  const request = useRequestExport()
  const job = useExportJob(exportId)
  const idPrefix = useId()
  const customLabel = useCustomObjectLabel(recordType)
  const typeLabel = isExportable(recordType) ? t(`recordTypes.${recordType}`) : (customLabel ?? recordType)

  const chosen = selected ?? new Set(config?.columns ?? [])
  const columns = config ? orderedColumns(config.columns, chosen, fields) : []

  const toggle = (key: string, checked: boolean) => {
    const next = new Set(chosen)
    if (checked) next.add(key)
    else next.delete(key)
    setSelected(next)
  }

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (!config || columns.length === 0) return
    request.mutate(
      {
        record_type: recordType,
        format,
        filter: config.filter ?? null,
        sort: sortToParam(config.sort) || null,
        search: search.trim() || null,
        archived: archived ?? null,
        columns: includeId ? [ID_COLUMN, ...columns] : columns,
      },
      { onSuccess: (created) => setExportId(created.id) },
    )
  }

  const status = job.data?.status ?? request.data?.status
  const finished = status === 'completed' || status === 'failed'

  return (
    <DialogContent className="max-h-[90dvh] w-[calc(100%-2rem)] max-w-lg overflow-y-auto">
      <DialogHeader>
        <DialogTitle>{t('title', { recordType: typeLabel })}</DialogTitle>
        <DialogDescription>{t('description')}</DialogDescription>
      </DialogHeader>
      {exportId ? (
        <div className="grid gap-3" role="status" aria-live="polite">
          {!finished && (
            <p className="flex items-center gap-2 text-sm">
              <LoaderCircle className="size-4 animate-spin" aria-hidden />
              {t('processing')}
            </p>
          )}
          {status === 'completed' && job.data?.file_id && (
            <>
              <p className="text-sm">{t('ready', { count: job.data.row_count ?? 0 })}</p>
              <Button asChild>
                <a href={fileDownloadUrl(job.data.file_id)} download>
                  <Download aria-hidden />
                  {t('download')}
                </a>
              </Button>
            </>
          )}
          {status === 'failed' && (
            <p role="alert" className="text-sm text-destructive">
              {job.data?.failure_reason ?? t('failed')}
            </p>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={onClose}>
              {t('close')}
            </Button>
          </DialogFooter>
        </div>
      ) : (
        <form className="grid gap-4" onSubmit={submit}>
          <fieldset className="grid gap-2">
            <legend className="mb-1 text-sm font-medium">{t('format')}</legend>
            <div className="flex flex-wrap gap-4">
              {FORMATS.map((option) => (
                <label key={option} className="flex items-center gap-2 text-sm">
                  <input
                    type="radio"
                    name={`${idPrefix}-format`}
                    value={option}
                    checked={format === option}
                    onChange={() => setFormat(option)}
                  />
                  {t(`formats.${option}`)}
                </label>
              ))}
            </div>
          </fieldset>
          <fieldset className="grid gap-2">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <legend className="text-sm font-medium">{t('columns')}</legend>
              <div className="flex gap-1">
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => setSelected(new Set(fields.map((field) => field.key)))}
                >
                  {t('selectAll')}
                </Button>
                <Button type="button" variant="ghost" size="sm" onClick={() => setSelected(new Set())}>
                  {t('selectNone')}
                </Button>
              </div>
            </div>
            <ul className="grid max-h-64 gap-2 overflow-y-auto rounded-md border p-3 sm:grid-cols-2">
              <li className="flex items-center gap-2">
                <Checkbox
                  id={`${idPrefix}-id`}
                  checked={includeId}
                  onCheckedChange={(checked) => setIncludeId(checked === true)}
                />
                <Label htmlFor={`${idPrefix}-id`} className="font-normal">
                  {t('includeId')}
                </Label>
              </li>
              {fields.map((field) => (
                <li key={field.key} className="flex min-w-0 items-center gap-2">
                  <Checkbox
                    id={`${idPrefix}-${field.key}`}
                    checked={chosen.has(field.key)}
                    onCheckedChange={(checked) => toggle(field.key, checked === true)}
                  />
                  <Label htmlFor={`${idPrefix}-${field.key}`} className="truncate font-normal">
                    {field.label}
                  </Label>
                </li>
              ))}
            </ul>
            {config && columns.length === 0 && (
              <p role="alert" className="text-sm text-destructive">
                {t('noColumns')}
              </p>
            )}
          </fieldset>
          {request.isError && (
            <p role="alert" className="text-sm text-destructive">
              {errorMessage(request.error)}
            </p>
          )}
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              {t('close')}
            </Button>
            <Button type="submit" disabled={!config || columns.length === 0 || request.isPending}>
              <FileDown aria-hidden />
              {request.isPending ? t('submitting') : t('submit')}
            </Button>
          </DialogFooter>
        </form>
      )}
    </DialogContent>
  )
}
