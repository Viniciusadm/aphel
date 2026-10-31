import { useMutation } from '@tanstack/react-query'
import { Loader2 } from 'lucide-react'
import { useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { UnsavedChangesDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Textarea } from '@/components/ui/textarea'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { emailApi, useSaveTemplate } from '../api'
import { insertAt, slugifyKey } from '../lib/format'
import { useEmailRecordTypes } from '../lib/record-types'
import type { EmailTemplate, EmailTemplateInput, TemplateScope } from '../types'
import { PreviewPane } from './PreviewPane'
import { VariablePicker } from './VariablePicker'

interface TemplateEditorDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  scope: TemplateScope
  template: EmailTemplate | null
}

export function TemplateEditorDialog({ open, onOpenChange, scope, template }: TemplateEditorDialogProps) {
  const { t } = useTranslation('email')
  const [confirmingClose, setConfirmingClose] = useState(false)
  const dirty = useRef(false)

  const requestClose = (next: boolean) => {
    if (next) return onOpenChange(true)
    if (dirty.current) setConfirmingClose(true)
    else onOpenChange(false)
  }

  return (
    <>
      <Dialog open={open} onOpenChange={requestClose}>
        <DialogContent size="2xl">
          <DialogHeader>
            <DialogTitle>{template ? t('templates.editTitle') : t('templates.createTitle')}</DialogTitle>
            <DialogDescription>{t(`templates.editorDescription.${scope}`)}</DialogDescription>
          </DialogHeader>
          {open && (
            <TemplateForm
              key={template?.id ?? 'new'}
              scope={scope}
              template={template}
              onDirtyChange={(value) => {
                dirty.current = value
              }}
              onCancel={() => requestClose(false)}
              onSaved={() => {
                dirty.current = false
                onOpenChange(false)
              }}
            />
          )}
        </DialogContent>
      </Dialog>
      <UnsavedChangesDialog
        open={confirmingClose}
        onStay={() => setConfirmingClose(false)}
        onLeave={() => {
          setConfirmingClose(false)
          dirty.current = false
          onOpenChange(false)
        }}
      />
    </>
  )
}

interface TemplateFormProps {
  scope: TemplateScope
  template: EmailTemplate | null
  onDirtyChange: (dirty: boolean) => void
  onCancel: () => void
  onSaved: () => void
}

function initialInput(scope: TemplateScope, template: EmailTemplate | null): EmailTemplateInput {
  return {
    key: template?.key ?? '',
    name: template?.name ?? '',
    scope,
    record_type: template?.record_type ?? null,
    subject: template?.subject ?? '',
    body_html: template?.body_html ?? '',
  }
}

type FocusTarget = 'subject' | 'body'

function TemplateForm({ scope, template, onDirtyChange, onCancel, onSaved }: TemplateFormProps) {
  const { t } = useTranslation('email')
  const formId = useId()
  const recordTypes = useEmailRecordTypes()
  const save = useSaveTemplate()
  const [initial] = useState(() => initialInput(scope, template))
  const [input, setInput] = useState(initial)
  const [keyEdited, setKeyEdited] = useState(Boolean(template))
  const [tab, setTab] = useState('edit')
  const subjectRef = useRef<HTMLInputElement>(null)
  const bodyRef = useRef<HTMLTextAreaElement>(null)
  const lastFocus = useRef<FocusTarget>('body')
  const preview = useMutation({ mutationFn: emailApi.preview })

  const update = (patch: Partial<EmailTemplateInput>) => {
    setInput((current) => {
      const next = { ...current, ...patch }
      onDirtyChange(JSON.stringify(next) !== JSON.stringify(initial))
      return next
    })
  }

  const insertVariable = (token: string) => {
    if (lastFocus.current === 'subject') {
      const element = subjectRef.current
      update({ subject: insertAt(input.subject, token, element?.selectionStart ?? null, element?.selectionEnd ?? null) })
      return
    }
    const element = bodyRef.current
    update({ body_html: insertAt(input.body_html, token, element?.selectionStart ?? null, element?.selectionEnd ?? null) })
  }

  const showPreview = (value: string) => {
    setTab(value)
    if (value === 'preview') preview.mutate({ subject: input.subject, body_html: input.body_html })
  }

  const valid = input.name.trim() !== '' && input.key.trim() !== '' && input.subject.trim() !== ''

  const submit = async () => {
    if (!valid) return
    try {
      await save.mutateAsync({
        templateId: template?.id ?? null,
        input: { ...input, name: input.name.trim(), key: input.key.trim(), subject: input.subject.trim() },
      })
      notifySuccess(template ? t('templates.updated') : t('templates.created'))
      onSaved()
    } catch (error) {
      notifyError(error)
    }
  }

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <div className="grid content-start gap-1.5">
          <Label htmlFor={`${formId}-name`}>{t('templates.name')}</Label>
          <Input placeholder={t('placeholders.templateName')}
            id={`${formId}-name`}
            value={input.name}
            required
            maxLength={120}
            onChange={(event) =>
              update(
                keyEdited
                  ? { name: event.target.value }
                  : { name: event.target.value, key: slugifyKey(event.target.value) },
              )
            }
          />
        </div>
        <div className="grid content-start gap-1.5">
          <Label htmlFor={`${formId}-key`}>{t('templates.key')}</Label>
          <Input placeholder={t('placeholders.templateKey')}
            id={`${formId}-key`}
            value={input.key}
            required
            maxLength={64}
            pattern="[a-z0-9_]+"
            className="font-mono"
            aria-describedby={`${formId}-key-hint`}
            onChange={(event) => {
              setKeyEdited(true)
              update({ key: slugifyKey(event.target.value) })
            }}
          />
          <p id={`${formId}-key-hint`} className="text-xs text-muted-foreground">
            {t('templates.keyHint')}
          </p>
        </div>
      </div>
      <div className="grid content-start gap-1.5">
        <Label htmlFor={`${formId}-record-type`}>{t('templates.recordType')}</Label>
        <NativeSelect
          id={`${formId}-record-type`}
          value={input.record_type ?? ''}
          onChange={(event) => update({ record_type: event.target.value || null })}
        >
          <option value="">{t('templates.anyRecordType')}</option>
          {recordTypes.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </NativeSelect>
      </div>
      <Tabs value={tab} onValueChange={showPreview} className="grid gap-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <TabsList>
            <TabsTrigger value="edit">{t('templates.editTab')}</TabsTrigger>
            <TabsTrigger value="preview">{t('templates.previewTab')}</TabsTrigger>
          </TabsList>
          {tab === 'edit' && <VariablePicker recordType={input.record_type} onInsert={insertVariable} />}
        </div>
        <TabsContent value="edit" className="grid gap-4">
          <div className="grid content-start gap-1.5">
            <Label htmlFor={`${formId}-subject`}>{t('fields.subject')}</Label>
            <Input placeholder={t('placeholders.subject')}
              id={`${formId}-subject`}
              ref={subjectRef}
              value={input.subject}
              required
              maxLength={300}
              onFocus={() => {
                lastFocus.current = 'subject'
              }}
              onChange={(event) => update({ subject: event.target.value })}
            />
          </div>
          <div className="grid content-start gap-1.5">
            <Label htmlFor={`${formId}-body`}>{t('templates.bodyHtml')}</Label>
            <Textarea placeholder={t('placeholders.bodyHtml')}
              id={`${formId}-body`}
              ref={bodyRef}
              value={input.body_html}
              rows={12}
              className="font-mono text-xs"
              aria-describedby={`${formId}-body-hint`}
              onFocus={() => {
                lastFocus.current = 'body'
              }}
              onChange={(event) => update({ body_html: event.target.value })}
            />
            <p id={`${formId}-body-hint`} className="text-xs text-muted-foreground">
              {t('templates.bodyHint', { example: '{{contact.first_name}}' })}
            </p>
          </div>
        </TabsContent>
        <TabsContent value="preview">
          <PreviewPane preview={preview.data} pending={preview.isPending} error={preview.isError} />
        </TabsContent>
      </Tabs>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onCancel}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" disabled={!valid || save.isPending}>
          {save.isPending && <Loader2 className="animate-spin" aria-hidden="true" />}
          {t('actions.save')}
        </Button>
      </DialogFooter>
    </form>
  )
}
