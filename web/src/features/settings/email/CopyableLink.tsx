import { Check, Copy } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Field } from '@/components/ui/field'

interface CopyableLinkProps {
  link: string
  label?: string
  description?: string
}

export function CopyableLink({ link, label, description }: CopyableLinkProps) {
  const { t } = useTranslation('settings')
  const id = useId()
  const [copied, setCopied] = useState(false)
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(link)
      setCopied(true)
    } catch {
      setCopied(false)
    }
  }
  return (
    <Field id={id} label={label ?? t('configuration.mail.copyableLinkLabel')}>
      <p className="text-sm text-muted-foreground">{description ?? t('configuration.mail.copyableLinkDescription')}</p>
      <div className="flex gap-2">
        <Input id={id} readOnly value={link} className="font-mono text-xs" onFocus={(event) => event.target.select()} />
        <Button type="button" variant="outline" onClick={() => void copy()}>
          {copied ? <Check aria-hidden /> : <Copy aria-hidden />}
          {copied ? t('configuration.mail.copied') : t('configuration.mail.copy')}
        </Button>
      </div>
    </Field>
  )
}
