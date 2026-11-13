import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { errorMessage } from '@/components/feedback'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { DialogForm } from '@/components/ui/dialog-form'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { slugProblem, suggestSlug, useBlocksValidation } from '@/sdk'
import { PAGE_TEMPLATES, pagesApi, type PageRecord, type PageTemplate } from './api'
import { PageWireframe, TEMPLATE_BLOCKS } from './PageWireframe'
import { useStarterTexts } from './useStarterTexts'

interface NewPageDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onCreated: (page: PageRecord) => void
}

export function NewPageDialog({ open, onOpenChange, onCreated }: NewPageDialogProps) {
  const { t } = useTranslation('site')
  const texts = useStarterTexts()
  const [title, setTitle] = useState('')
  const [slug, setSlug] = useState('')
  const [slugEdited, setSlugEdited] = useState(false)
  const [template, setTemplate] = useState<PageTemplate>('blank')
  const [pending, setPending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const effectiveSlug = slugEdited ? slug : suggestSlug(title, 'pagina')
  const validation = useBlocksValidation()
  const problem = validation.isSuccess && title.trim() ? slugProblem(effectiveSlug) : null

  function close(next: boolean) {
    if (!next) {
      setTitle('')
      setSlug('')
      setSlugEdited(false)
      setTemplate('blank')
      setError(null)
    }
    onOpenChange(next)
  }

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent>
        <DialogForm
          title={t('pages.create.title')}
          description={t('pages.create.description')}
          error={error}
          pending={pending}
          submitDisabled={!validation.isSuccess || !title.trim() || problem !== null}
          submitLabel={t('pages.create.submit')}
          cancelLabel={t('pages.create.cancel')}
          onCancel={() => close(false)}
          onSubmit={async (event) => {
            event.preventDefault()
            setPending(true)
            setError(null)
            try {
              const page = await pagesApi.create(title.trim(), template, effectiveSlug, slugEdited, texts)
              close(false)
              onCreated(page)
            } catch (failure) {
              setError(errorMessage(failure))
            } finally {
              setPending(false)
            }
          }}
        >
          <Field label={t('pages.create.fieldTitle')} required>
            {(control) => <Input {...control} autoFocus value={title} maxLength={160} placeholder={t('pages.create.fieldTitlePlaceholder')} onChange={(event) => setTitle(event.target.value)} />}
          </Field>
          <fieldset className="editorial-templates">
            <legend>{t('pages.templates.label')}</legend>
            {PAGE_TEMPLATES.map((option) => (
              <label key={option} className="editorial-template pages-template" data-checked={template === option || undefined}>
                <input type="radio" name="editorial-template" value={option} checked={template === option} onChange={() => setTemplate(option)} />
                <PageWireframe types={TEMPLATE_BLOCKS[option]} />
                <span>
                  <strong>{t(`pages.templates.${option}`)}</strong>
                  <small>{t(`pages.templates.${option}Help`)}</small>
                </span>
              </label>
            ))}
          </fieldset>
          <Field
            label={t('pages.create.fieldSlug')}
            description={t('pages.create.fieldSlugHelp', { url: `…/${effectiveSlug || ''}` })}
            error={problem ? t(`pages.slug.${problem}`) : undefined}
          >
            {(control) => (
              <div className="editorial-slug">
                <span aria-hidden>/</span>
                <Input
                  {...control}
                  value={effectiveSlug}
                  maxLength={80}
                  autoComplete="off"
                  spellCheck={false}
                  onChange={(event) => {
                    setSlugEdited(true)
                    setSlug(event.target.value.toLowerCase())
                  }}
                />
              </div>
            )}
          </Field>
        </DialogForm>
      </DialogContent>
    </Dialog>
  )
}
