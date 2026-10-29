import { useQueryClient } from '@tanstack/react-query'
import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { NETWORK_ERROR_CODE, isApiError } from '@/api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { AuthCard } from '@/features/auth/components/AuthCard'
import { INSTALLER_STATUS_KEY, restoreBackup } from './api'

export function RestoreStep({ onCancel }: { onCancel: () => void }) {
  const { t } = useTranslation('installer')
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const inputId = useId()
  const [file, setFile] = useState<File | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [submitting, setSubmitting] = useState(false)

  const onSubmit = async (event: FormEvent) => {
    event.preventDefault()
    if (!file) {
      setError(t('restore.errors.fileRequired'))
      return
    }
    setError(null)
    setSubmitting(true)
    try {
      await restoreBackup(file)
      queryClient.setQueryData(INSTALLER_STATUS_KEY, false)
      await navigate('/login', { replace: true, state: { restored: true } })
    } catch (failure) {
      if (isApiError(failure) && failure.status === 404) {
        queryClient.setQueryData(INSTALLER_STATUS_KEY, false)
        await navigate('/login', { replace: true })
        return
      }
      if (isApiError(failure) && failure.code === NETWORK_ERROR_CODE) setError(t('errors.network'))
      else if (isApiError(failure) && failure.status === 413) setError(t('restore.errors.tooLarge'))
      else if (isApiError(failure) && failure.status === 422) setError(t('restore.errors.invalid', { message: failure.message }))
      else setError(t('restore.errors.generic'))
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <AuthCard title={t('restore.title')} description={t('restore.description')}>
      <form className="grid gap-4" onSubmit={(event) => void onSubmit(event)} noValidate>
        {error && (
          <p role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
            {error}
          </p>
        )}
        <div className="grid gap-2">
          <Label htmlFor={inputId}>{t('restore.file')}</Label>
          <Input
            id={inputId}
            type="file"
            accept=".zip,application/zip"
            disabled={submitting}
            onChange={(event) => setFile(event.target.files?.[0] ?? null)}
          />
          <p className="text-xs text-muted-foreground">{t('restore.hint')}</p>
        </div>
        <Button type="submit" className="w-full" disabled={submitting}>
          {submitting ? t('restore.submitting') : t('restore.submit')}
        </Button>
        <Button type="button" variant="ghost" className="w-full" disabled={submitting} onClick={onCancel}>
          {t('restore.back')}
        </Button>
      </form>
    </AuthCard>
  )
}
