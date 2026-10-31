import { Mic, MicOff } from 'lucide-react'
import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast-store'
import { cn } from '@/lib/utils'
import { useDictation, type DictationFailure } from '../lib/speech'
import { Field } from '@/components/ui/field'

export interface DictationFieldProps {
  id: string
  label: string
  value: string
  onChange: (value: string) => void
  placeholder?: string
  maxLength?: number
}

function appendText(current: string, addition: string): string {
  if (!current.trim()) return addition
  return /\s$/.test(current) ? `${current}${addition}` : `${current} ${addition}`
}

export function DictationField({ id, label, value, onChange, placeholder, maxLength }: DictationFieldProps) {
  const { t } = useTranslation('mobile')
  const onText = useCallback((text: string) => onChange(appendText(value, text)), [onChange, value])
  const onFailure = useCallback(
    (failure: DictationFailure) => toast({ title: t(`dictation.failure.${failure}`), variant: 'destructive' }),
    [t],
  )
  const dictation = useDictation({ onText, onFailure })

  return (
    <Field
      id={id}
      label={label}
      labelAction={
        dictation.supported ? (
          <Button
            type="button"
            variant={dictation.listening ? 'destructive' : 'outline'}
            size="sm"
            className="h-9"
            aria-pressed={dictation.listening}
            onClick={dictation.listening ? dictation.stop : dictation.start}
          >
            {dictation.listening ? <MicOff aria-hidden="true" /> : <Mic aria-hidden="true" />}
            {t(dictation.listening ? 'dictation.stop' : 'dictation.start')}
          </Button>
        ) : (
          <span className="text-xs text-muted-foreground">{t('dictation.unsupported')}</span>
        )
      }
    >
      <Textarea
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        maxLength={maxLength}
        className={cn('min-h-32 text-base', dictation.listening && 'ring-2 ring-destructive/40')}
      />
      {dictation.listening && (
        <p className="text-xs text-muted-foreground" aria-live="polite">
          {dictation.interim || t('dictation.listening')}
        </p>
      )}
    </Field>
  )
}
