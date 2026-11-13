import { useQuery } from '@tanstack/react-query'
import { Check, Copy } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { useModulesApi, type ModuleInfo } from '@/kernel/modules/api'
import { useModuleText } from './text'

function HintRow({ id, label, help, value }: { id: string; label: string; help: string; value: string }) {
  const { t } = useTranslation('modules')
  const [copied, setCopied] = useState(false)
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(true)
      window.setTimeout(() => setCopied(false), 2000)
    } catch (error) {
      notifyError(error)
    }
  }
  return (
    <Field id={id} label={label}>
      {(control) => (
        <>
          <div className="flex gap-2">
            <Input {...control} readOnly value={value} className="min-w-0 font-mono text-xs" />
            <Button type="button" variant="outline" size="sm" onClick={() => void copy()}>
              {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
              <span className="sr-only sm:not-sr-only">{copied ? t('settings.hintCopied') : t('settings.hintCopy')}</span>
            </Button>
          </div>
          {help && <p className="text-xs text-muted-foreground">{help}</p>}
        </>
      )}
    </Field>
  )
}

export function SetupHints({ module, idPrefix }: { module: ModuleInfo; idPrefix: string }) {
  const modulesApi = useModulesApi()
  const text = useModuleText(module)
  const hints = useQuery({
    queryKey: ['modules', 'setup-hints', module.key],
    queryFn: () => modulesApi.setupHints?.(module.key) ?? Promise.resolve([]),
    enabled: module.has_setup_hints && Boolean(modulesApi.setupHints),
    staleTime: 60_000,
  })
  if (!hints.data || hints.data.length === 0) return null
  return (
    <section className="grid gap-3 rounded-md border bg-muted/30 p-3">
      {hints.data.map((hint) => (
        <HintRow key={hint.key} id={`${idPrefix}-hint-${hint.key}`} label={text.hintLabel(hint.key, hint.key)} help={text.hintHelp(hint.key)} value={hint.value} />
      ))}
    </section>
  )
}
