import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { Upload } from 'lucide-react'
import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { errorMessage } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { useUploadImport } from '../api'
import { Field } from '@/components/ui/field'

export interface UploadStepProps {
  recordType: string
  retryOf?: string
}

export function UploadStep({ recordType, retryOf }: UploadStepProps) {
  const { t } = useTranslation('imports')
  const navigate = useNavigate()
  const inputId = useId()
  const upload = useUploadImport()
  const [file, setFile] = useState<File | null>(null)
  const [missing, setMissing] = useState(false)

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (!file) {
      setMissing(true)
      return
    }
    upload.mutate(
      { recordType, file, retryOf },
      { onSuccess: (job) => void navigate(`/imports/${encodeURIComponent(job.id)}`) },
    )
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('upload.title')}</CardTitle>
        <CardDescription>{t('upload.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        <form className="grid gap-4" onSubmit={submit} noValidate>
          <Field id={inputId} label={t('upload.fileLabel')}>
            <input
              id={inputId}
              type="file"
              accept=".csv,.xlsx,.xls,.ods,text/csv,text/plain,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,application/vnd.ms-excel,application/vnd.oasis.opendocument.spreadsheet"
              className="block w-full min-w-0 text-sm file:mr-3 file:rounded-md file:border file:border-input file:bg-background file:px-3 file:py-1.5 file:text-sm"
              onChange={(event) => {
                setFile(event.target.files?.[0] ?? null)
                setMissing(false)
              }}
            />
          </Field>
          {missing && (
            <SharedFieldError>
              {t('upload.required')}
            </SharedFieldError>
          )}
          {upload.isError && (
            <SharedFieldError>
              {t('errors.upload')} {errorMessage(upload.error)}
            </SharedFieldError>
          )}
          <div>
            <Button type="submit" disabled={upload.isPending}>
              <Upload aria-hidden />
              {upload.isPending ? t('upload.uploading') : t('upload.submit')}
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  )
}
