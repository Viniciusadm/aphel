import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { errorMessage } from '@/components/feedback'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { DialogForm } from '@/components/ui/dialog-form'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast-store'
import { readText, slugProblem, useBlocksValidation, writeText, type DocumentSettingsProps } from '@/sdk'
import { pagesApi } from './api'

export function PageSettingsDialog({ language, open, record: page, onOpenChange, onSaved }: DocumentSettingsProps) {
  const { t } = useTranslation('site')
  const [slug, setSlug] = useState(page.slug)
  const [description, setDescription] = useState(readText(page, 'draft_seo_description', language))
  const [pending, setPending] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!open) return
    setSlug(page.slug)
    setDescription(readText(page, 'draft_seo_description', language))
    setError(null)
  }, [open, page, language])

  const validation = useBlocksValidation()
  const problem = validation.isSuccess ? slugProblem(slug) : null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg">
        <DialogForm
          title={t('pages.settings.title')}
          description={t('pages.settings.description')}
          error={error}
          pending={pending}
          submitDisabled={!validation.isSuccess || problem !== null}
          submitLabel={t('pages.settings.save')}
          cancelLabel={t('pages.settings.cancel')}
          onCancel={() => onOpenChange(false)}
          onSubmit={async (event) => {
            event.preventDefault()
            setPending(true)
            setError(null)
            try {
              const updated = await pagesApi.update(page.id, { slug, ...writeText(page, 'draft_seo_description', language, description.trim()) })
              onSaved(updated)
              toast({ title: t('pages.settings.saved'), variant: 'success' })
              onOpenChange(false)
            } catch (failure) {
              setError(errorMessage(failure))
            } finally {
              setPending(false)
            }
          }}
        >
          <Field
            label={t('pages.settings.slug')}
            required
            description={page.publication_state === 'published' ? t('pages.settings.slugPublishedHelp') : undefined}
            error={problem ? t(`pages.slug.${problem}`) : undefined}
          >
            {(control) => (
              <div className="editorial-slug">
                <span aria-hidden>/</span>
                <Input {...control} value={slug} maxLength={80} autoComplete="off" spellCheck={false} onChange={(event) => setSlug(event.target.value.toLowerCase())} />
              </div>
            )}
          </Field>
          <Field label={t('pages.settings.seoDescription')} description={t('pages.settings.seoDescriptionHelp')}>
            {(control) => <Textarea {...control} rows={3} maxLength={300} value={description} onChange={(event) => setDescription(event.target.value)} />}
          </Field>
        </DialogForm>
      </DialogContent>
    </Dialog>
  )
}
