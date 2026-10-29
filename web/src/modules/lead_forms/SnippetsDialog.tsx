import { Copy, ExternalLink } from 'lucide-react'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { LEAD_FORMS_KEY, absoluteUrl, iframeSnippet, scriptSnippet, type LeadForm } from './api'

interface SnippetsDialogProps {
  form: LeadForm | null
  onClose: () => void
}

export function SnippetsDialog({ form, onClose }: SnippetsDialogProps) {
  const t = useModuleTranslation(LEAD_FORMS_KEY)
  return (
    <Dialog open={form !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-h-[92vh] overflow-y-auto sm:max-w-3xl">
        {form && (
          <>
            <DialogHeader>
              <DialogTitle>{t('snippets.title', { name: form.name })}</DialogTitle>
              <DialogDescription>{t('snippets.description')}</DialogDescription>
            </DialogHeader>
            {!form.is_active && (
              <p className="rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm">
                {t('snippets.inactive')}
              </p>
            )}
            <Snippet label={t('snippets.iframe')} help={t('snippets.iframeHelp')} code={iframeSnippet(form)} />
            <Snippet label={t('snippets.script')} help={t('snippets.scriptHelp')} code={scriptSnippet(form)} />
            <Snippet label={t('snippets.link')} help={t('snippets.linkHelp')} code={absoluteUrl(form.page_path)} />
            <div className="flex justify-end">
              <Button asChild size="sm" variant="outline">
                <a href={absoluteUrl(form.page_path)} target="_blank" rel="noreferrer">
                  <ExternalLink aria-hidden="true" />
                  {t('snippets.open')}
                </a>
              </Button>
            </div>
            {form.is_active && (
              <section className="grid gap-2">
                <h3 className="text-sm font-semibold">{t('snippets.livePreview')}</h3>
                <iframe
                  src={form.page_path}
                  title={t('snippets.livePreviewTitle', { name: form.name })}
                  className="h-[420px] w-full rounded-md border bg-white"
                  sandbox=""
                />
              </section>
            )}
          </>
        )}
      </DialogContent>
    </Dialog>
  )
}

interface SnippetProps {
  label: string
  help: string
  code: string
}

function Snippet({ label, help, code }: SnippetProps) {
  const t = useModuleTranslation(LEAD_FORMS_KEY)
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code)
      notifySuccess(t('snippets.copied'))
    } catch (error) {
      notifyError(error)
    }
  }
  return (
    <section className="grid gap-1.5">
      <div className="flex items-center justify-between gap-2">
        <h3 className="text-sm font-semibold">{label}</h3>
        <Button type="button" size="sm" variant="outline" onClick={() => void copy()}>
          <Copy aria-hidden="true" />
          {t('snippets.copy')}
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">{help}</p>
      <pre className="overflow-x-auto rounded-md bg-muted p-3 text-xs whitespace-pre-wrap break-all">
        <code>{code}</code>
      </pre>
    </section>
  )
}
