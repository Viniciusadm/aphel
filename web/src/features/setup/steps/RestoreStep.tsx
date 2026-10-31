import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { NETWORK_ERROR_CODE, isApiError } from '@/api'
import { Input } from '@/components/ui/input'
import { SetupStepFrame } from '@/kernel/setup'
import { INSTALLER_STATUS_KEY, restoreBackup } from '../api'
import { Field } from '@/components/ui/field'

export function RestoreStep() {
  const { t } = useTranslation('setup')
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [file, setFile] = useState<File | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [submitting, setSubmitting] = useState(false)

  const submit = async () => {
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
    <SetupStepFrame
      title={t('restore.title')}
      description={t('restore.description')}
      error={error}
      primary={{ label: t('restore.submit'), onClick: () => void submit(), busy: submitting, busyLabel: t('restore.submitting') }}
    >
      <Field label={t('restore.file')}>
        {(control) => <>
          <Input
          {...control}
          type="file"
          accept=".zip,application/zip"
          disabled={submitting}
          onChange={(event) => setFile(event.target.files?.[0] ?? null)}
        />
        <p className="text-xs text-muted-foreground">{t('restore.hint')}</p>
        </>}
      </Field>
    </SetupStepFrame>
  )
}
