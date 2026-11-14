import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { Download, FileDown, LoaderCircle } from 'lucide-react'
import { type ArchivedMode, fileDownloadUrl, sortToParam, useCustomObjectLabel } from '@/sdk'
import { useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { errorMessage } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { isExportable, useExportJob, useRequestExport, type ExportFormat } from './api'
import { orderedColumns, useCurrentListConfig } from './exportColumns'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { ToggleField } from '@/components/ui/toggle-field'

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
    <DialogContent size="md">
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
            <SharedFieldError>
              {t('failed')}
            </SharedFieldError>
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
            <SegmentedControl
              aria-label={t('format')}
              value={format}
              onValueChange={setFormat}
              options={FORMATS.map((option) => ({ value: option, label: t(`formats.${option}`) }))}
            />
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
            <div className="grid max-h-64 gap-2 overflow-y-auto rounded-md border p-3 sm:grid-cols-2">
              <ToggleField label={t('includeId')} checked={includeId} onCheckedChange={(checked) => setIncludeId(checked === true)} />
              {fields.map((field) => (
                <ToggleField key={field.key} label={field.label} checked={chosen.has(field.key)} onCheckedChange={(checked) => toggle(field.key, checked === true)} className="min-w-0" />
              ))}
            </div>
            {config && columns.length === 0 && (
              <SharedFieldError>
                {t('noColumns')}
              </SharedFieldError>
            )}
          </fieldset>
          {request.isError && (
            <SharedFieldError>
              {errorMessage(request.error)}
            </SharedFieldError>
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
