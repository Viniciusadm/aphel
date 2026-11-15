import { Paperclip, X } from 'lucide-react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { errorMessage } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { fileDownloadUrl, uploadFile } from '@/features/files'
import { ariaProps, type FieldInputBaseProps } from './types'

export interface FileFieldInputProps extends FieldInputBaseProps<string> {
  accept?: string
  clearable?: boolean
}

export function FileFieldInput({ accept, clearable = true, ...props }: FileFieldInputProps) {
  const { t } = useTranslation('records-form')
  const input = useRef<HTMLInputElement>(null)
  const [uploading, setUploading] = useState(false)
  const [name, setName] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const choose = async (file: File | undefined) => {
    if (!file) return
    setUploading(true)
    setError(null)
    try {
      const uploaded = await uploadFile({ file })
      setName(uploaded.file.original_name)
      props.onChange(uploaded.file.id)
    } catch (failure) {
      setError(errorMessage(failure))
    } finally {
      setUploading(false)
    }
  }

  return (
    <div className="grid min-w-0 gap-1">
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <input
          ref={input}
          type="file"
          accept={accept}
          className="sr-only"
          tabIndex={-1}
          aria-hidden="true"
          onChange={(event) => {
            void choose(event.target.files?.[0])
            event.target.value = ''
          }}
        />
        <Button
          type="button"
          variant="outline"
          size="sm"
          {...ariaProps(props)}
          disabled={props.disabled || uploading}
          onClick={() => input.current?.click()}
        >
          <Paperclip aria-hidden="true" />
          {uploading ? t('input.fileUploading') : props.value ? t('input.fileReplace') : t('input.fileChoose')}
        </Button>
        {props.value && (
          <>
            <a
              href={fileDownloadUrl(props.value)}
              target="_blank"
              rel="noreferrer noopener"
              className="min-w-0 truncate text-sm text-primary underline-offset-4 hover:underline"
            >
              {name ?? t('input.fileCurrent')}
            </a>
            {clearable && (
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="size-8"
                disabled={props.disabled || uploading}
                aria-label={t('input.fileRemove')}
                onClick={() => {
                  setName(null)
                  props.onChange(null)
                }}
              >
                <X aria-hidden="true" />
              </Button>
            )}
          </>
        )}
      </div>
      {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
    </div>
  )
}
