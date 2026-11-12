import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { AlertTriangle, CheckCircle2, Download } from 'lucide-react'
import { useId, useState } from 'react'
import { Trans, useTranslation } from 'react-i18next'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { useModulesApi, type ModuleInfo } from '@/kernel/modules/api'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { moduleErrorMessage, notifyModuleError } from './errors'
import { exportCoversDisable, exportModuleData } from './export'
import { Alert } from '@/components/ui/alert'
import { Field } from '@/components/ui/field'

interface PurgeDataDialogProps {
  module: ModuleInfo
  name: string
  onClose: () => void
  onChanged: () => Promise<unknown>
}

export function PurgeDataDialog({ module, name, onClose, onChanged }: PurgeDataDialogProps) {
  const { t } = useTranslation('modules')
  const modulesApi = useModulesApi()
  const formatting = useRecordFormatting()
  const inputId = useId()
  const errorId = useId()
  const [typed, setTyped] = useState('')
  const [exportedAt, setExportedAt] = useState<string | null>(null)
  const [exporting, setExporting] = useState(false)
  const [purging, setPurging] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const exported = exportCoversDisable(module, exportedAt)
  const lastExport = exportedAt ?? module.data_exported_at
  const typedMatches = typed.trim() === module.key
  const busy = exporting || purging

  const runExport = async () => {
    setExporting(true)
    setError(null)
    try {
      const result = await exportModuleData(module.key, modulesApi)
      setExportedAt(result.exported_at)
      notifySuccess(t('toast.exported', { name }))
      await onChanged()
    } catch (exportError) {
      notifyModuleError(exportError)
    } finally {
      setExporting(false)
    }
  }

  const purge = async () => {
    if (!exported || !typedMatches) return
    setPurging(true)
    setError(null)
    try {
      await modulesApi.purgeData(module.key, typed.trim())
      notifySuccess(t('toast.purged', { name }))
      await onChanged()
      onClose()
    } catch (purgeError) {
      const message = moduleErrorMessage(purgeError)
      if (message) setError(message)
      else notifyModuleError(purgeError)
      await onChanged()
    } finally {
      setPurging(false)
    }
  }

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={!busy} aria-busy={busy}>
        <DialogHeader>
          <DialogTitle>{t('purge.title', { name })}</DialogTitle>
          <DialogDescription>{t('purge.description')}</DialogDescription>
        </DialogHeader>
        <Alert
          role="note" className="flex gap-2 text-destructive"
        >
          <AlertTriangle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
          <p>{t('purge.warning')}</p>
        </Alert>
        <div className="grid gap-2 rounded-md border px-3 py-2 text-sm">
          {exported && lastExport ? (
            <p className="flex items-center gap-2">
              <CheckCircle2 className="size-4 shrink-0 text-success" aria-hidden="true" />
              {t('purge.exportDone', { date: formatDateTime(lastExport, formatting) })}
            </p>
          ) : (
            <p>{t('purge.exportStep')}</p>
          )}
          <Button
            type="button"
            size="sm"
            variant={exported ? 'outline' : 'default'}
            className="w-fit"
            disabled={busy}
            aria-busy={exporting}
            onClick={() => void runExport()}
          >
            <Download aria-hidden="true" />
            {t('purge.exportAction')}
          </Button>
        </div>
        <form
          onSubmit={(event) => {
            event.preventDefault()
            void purge()
          }}
        >
          <Field
            id={inputId}
            label={
              <Trans
                t={t}
                i18nKey="purge.typeToConfirm"
                values={{ key: module.key }}
                components={{ strong: <strong className="font-semibold break-all" /> }}
              />
            }
          >
          <Input placeholder={module.key}
            id={inputId}
            value={typed}
            autoComplete="off"
            autoCapitalize="off"
            spellCheck={false}
            disabled={busy || !exported}
            aria-invalid={error ? true : undefined}
            aria-describedby={error ? errorId : undefined}
            onChange={(event) => setTyped(event.target.value)}
          />
          {error && (
            <SharedFieldError id={errorId}>
              {error}
            </SharedFieldError>
          )}
          </Field>
        </form>
        <DialogFooter>
          <Button type="button" variant="outline" disabled={busy} onClick={onClose}>
            {t('purge.cancel')}
          </Button>
          <Button
            type="button"
            variant="destructive"
            disabled={busy || !exported || !typedMatches}
            aria-busy={purging}
            onClick={() => void purge()}
          >
            {purging ? t('purge.pending') : t('purge.confirm')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
