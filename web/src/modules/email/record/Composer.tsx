import { useMutation } from '@tanstack/react-query'
import { CalendarClock, Loader2, Paperclip, Send, X } from 'lucide-react'
import { Suspense, lazy, useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { UnsavedChangesDialog, notifySuccess } from '@/components/feedback'
import { DateTimeFieldInput } from '@/components/fields'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
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
import { uploadFile } from '@/features/files'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { emailApi, useEmailTemplates, useInvalidateThreads, useTrackingSettings } from '../api'
import { PreviewPane } from '../components/PreviewPane'
import { VariablePicker } from '../components/VariablePicker'
import { notifyEmailError } from '../lib/errors'
import { formatFileSize, hasVariables, insertAt, replyRecipients, replySubject, textToHtml } from '../lib/format'
import type { EmailAddress, EmailMessage, EmailRecordRef, EmailThread, Mailbox, SendEmailRequest } from '../types'
import { AddressField } from './AddressField'
import { htmlToText } from './html-text'
import type { RichEmailEditorHandle } from './RichEmailEditor'
import { useRecipientSuggestions } from './suggestions'

const RichEmailEditor = lazy(() => import('./RichEmailEditor'))

export interface ReplyContext {
  thread: EmailThread
  message: EmailMessage
}

interface ComposerDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  record: EmailRecordRef
  mailboxes: Mailbox[]
  reply: ReplyContext | null
}

export function ComposerDialog({ open, onOpenChange, record, mailboxes, reply }: ComposerDialogProps) {
  const { t } = useTranslation('email')
  const dirty = useRef(false)
  const [confirmingClose, setConfirmingClose] = useState(false)

  const requestClose = (next: boolean) => {
    if (next) return
    if (dirty.current) setConfirmingClose(true)
    else onOpenChange(false)
  }

  const close = () => {
    dirty.current = false
    setConfirmingClose(false)
    onOpenChange(false)
  }

  return (
    <>
      <Dialog open={open} onOpenChange={requestClose}>
        <DialogContent size="2xl" className="[--dialog-padding:--spacing(4)] sm:[--dialog-padding:--spacing(6)]">
          <DialogHeader>
            <DialogTitle>{reply ? t('composer.replyTitle') : t('composer.title')}</DialogTitle>
            <DialogDescription>{t('composer.description')}</DialogDescription>
          </DialogHeader>
          {open && (
            <ComposerForm
              key={reply?.message.id ?? 'new'}
              record={record}
              mailboxes={mailboxes}
              reply={reply}
              onDirtyChange={(value) => {
                dirty.current = value
              }}
              onCancel={() => requestClose(false)}
              onSent={close}
            />
          )}
        </DialogContent>
      </Dialog>
      <UnsavedChangesDialog open={confirmingClose} onStay={() => setConfirmingClose(false)} onLeave={close} />
    </>
  )
}

type BodyFormat = 'rich' | 'text' | 'html'

const BODY_FORMATS: BodyFormat[] = ['rich', 'text', 'html']

interface PendingAttachment {
  key: string
  name: string
  size: number
  fileId: string | null
  progress: number
  failed: boolean
}

interface ComposerFormProps {
  record: EmailRecordRef
  mailboxes: Mailbox[]
  reply: ReplyContext | null
  onDirtyChange: (dirty: boolean) => void
  onCancel: () => void
  onSent: () => void
}

function initialMailbox(mailboxes: Mailbox[], reply: ReplyContext | null): string {
  if (reply && mailboxes.some((mailbox) => mailbox.id === reply.thread.mailbox_id)) return reply.thread.mailbox_id
  return (mailboxes.find((mailbox) => mailbox.status !== 'disconnected') ?? mailboxes[0])?.id ?? ''
}

function ComposerForm({ record, mailboxes, reply, onDirtyChange, onCancel, onSent }: ComposerFormProps) {
  const { t } = useTranslation('email')
  const formId = useId()
  const formatting = useRecordFormatting()
  const invalidateThreads = useInvalidateThreads()
  const suggestions = useRecipientSuggestions(record)
  const templates = useEmailTemplates('all')
  const tracking = useTrackingSettings()
  const trackingAvailable = Boolean(tracking.data && (tracking.data.opens_enabled || tracking.data.clicks_enabled))

  const [mailboxId, setMailboxId] = useState(() => initialMailbox(mailboxes, reply))
  const [to, setTo] = useState<EmailAddress[]>(() =>
    reply ? replyRecipients(reply.message, reply.thread.mailbox_address) : [],
  )
  const [cc, setCc] = useState<EmailAddress[]>([])
  const [bcc, setBcc] = useState<EmailAddress[]>([])
  const [showCopies, setShowCopies] = useState(false)
  const [subject, setSubject] = useState(() => (reply ? replySubject(reply.thread.subject) : ''))
  const [body, setBody] = useState('')
  const [format, setFormat] = useState<BodyFormat>('rich')
  const [template, setTemplate] = useState<{ id: string; subject: string; body: string } | null>(null)
  const [attachments, setAttachments] = useState<PendingAttachment[]>([])
  const [scheduled, setScheduled] = useState(false)
  const [scheduledAt, setScheduledAt] = useState<string | null>(null)
  const [trackingOverride, setTrackingOverride] = useState<boolean | null>(null)
  const [tab, setTab] = useState('write')
  const [submitted, setSubmitted] = useState(false)
  const subjectRef = useRef<HTMLInputElement>(null)
  const bodyRef = useRef<HTMLTextAreaElement>(null)
  const richRef = useRef<RichEmailEditorHandle>(null)
  const fileInput = useRef<HTMLInputElement>(null)
  const lastFocus = useRef<'subject' | 'body'>('body')

  const trackingEnabled = trackingAvailable && (trackingOverride ?? tracking.data?.default_on ?? false)
  const bodyHtml = format === 'text' ? textToHtml(body) : body
  const uploading = attachments.some((attachment) => attachment.fileId === null && !attachment.failed)
  const [scheduleInvalid, setScheduleInvalid] = useState(false)
  const recipientsMissing = to.length === 0
  const availableTemplates = (templates.data ?? []).filter(
    (item) => item.record_type === null || item.record_type === record.record_type,
  )

  const markDirty = () => onDirtyChange(true)

  const preview = useMutation({ mutationFn: emailApi.preview })
  const applyTemplate = useMutation({
    mutationFn: (templateId: string) => emailApi.preview({ template_id: templateId, record }),
    onSuccess: (result, templateId) => {
      setSubject(reply ? subject : result.subject)
      setBody(result.html)
      setFormat('html')
      setTemplate({ id: templateId, subject: reply ? subject : result.subject, body: result.html })
      markDirty()
    },
    onError: (error) => notifyEmailError(error),
  })

  const insertVariable = (token: string) => {
    markDirty()
    if (lastFocus.current === 'subject') {
      const element = subjectRef.current
      setSubject(insertAt(subject, token, element?.selectionStart ?? null, element?.selectionEnd ?? null))
      return
    }
    if (format === 'rich') {
      richRef.current?.insertText(token)
      return
    }
    const element = bodyRef.current
    setBody(insertAt(body, token, element?.selectionStart ?? null, element?.selectionEnd ?? null))
  }

  const changeTab = (value: string) => {
    setTab(value)
    if (value === 'preview') preview.mutate({ subject, body_html: bodyHtml, record })
  }

  const addFiles = (files: FileList | null) => {
    if (!files) return
    markDirty()
    for (const file of Array.from(files)) {
      const key = `${file.name}-${file.size}-${Math.random().toString(36).slice(2)}`
      setAttachments((current) => [
        ...current,
        { key, name: file.name, size: file.size, fileId: null, progress: 0, failed: false },
      ])
      const patch = (changes: Partial<PendingAttachment>) =>
        setAttachments((current) => current.map((item) => (item.key === key ? { ...item, ...changes } : item)))
      uploadFile({ file, onProgress: (progress) => patch({ progress }) })
        .then((uploaded) => patch({ fileId: uploaded.file.id, progress: 1 }))
        .catch((error: unknown) => {
          patch({ failed: true })
          notifyEmailError(error)
        })
    }
  }

  const send = useMutation({
    mutationFn: async () => {
      let finalSubject = subject.trim()
      let finalHtml = bodyHtml
      let finalText = format === 'text' ? body : format === 'rich' ? htmlToText(body) || undefined : undefined
      if (hasVariables(finalSubject) || hasVariables(finalHtml)) {
        const rendered = await emailApi.preview({ subject: finalSubject, body_html: finalHtml, record })
        finalSubject = rendered.subject
        finalHtml = rendered.html
        finalText = rendered.text
      }
      const untouchedTemplate = template && template.subject === subject && template.body === body
      const request: SendEmailRequest = {
        mailbox_id: mailboxId || undefined,
        to,
        cc: cc.length > 0 ? cc : undefined,
        bcc: bcc.length > 0 ? bcc : undefined,
        subject: finalSubject,
        body_html: finalHtml,
        body_text: finalText,
        template_id: untouchedTemplate ? template.id : undefined,
        record,
        reply_to_message_id: reply?.message.id,
        attachment_file_ids: attachments.flatMap((attachment) => (attachment.fileId ? [attachment.fileId] : [])),
        scheduled_at: scheduled && scheduledAt ? scheduledAt : undefined,
        tracking: trackingAvailable ? trackingEnabled : undefined,
      }
      return emailApi.send(request)
    },
    onSuccess: async (message) => {
      onDirtyChange(false)
      notifySuccess(
        scheduled && message.status === 'scheduled' && message.scheduled_at
          ? t('composer.scheduled', { date: formatDateTime(message.scheduled_at, formatting) })
          : t('composer.queued'),
      )
      await invalidateThreads()
      onSent()
    },
    onError: (error) => notifyEmailError(error),
  })

  const submit = () => {
    setSubmitted(true)
    const invalidSchedule = scheduled && (!scheduledAt || new Date(scheduledAt).getTime() <= Date.now())
    setScheduleInvalid(invalidSchedule)
    if (recipientsMissing || subject.trim() === '' || invalidSchedule || uploading) return
    send.mutate()
  }

  return (
    <form
      className="grid gap-4"
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        submit()
      }}
    >
      {mailboxes.length > 1 && (
        <div className="grid content-start gap-1.5">
          <Label htmlFor={`${formId}-mailbox`}>{t('composer.from')}</Label>
          <NativeSelect
            id={`${formId}-mailbox`}
            value={mailboxId}
            disabled={Boolean(reply)}
            onChange={(event) => setMailboxId(event.target.value)}
          >
            {mailboxes.map((mailbox) => (
              <option key={mailbox.id} value={mailbox.id}>
                {mailbox.display_name ? `${mailbox.display_name} <${mailbox.address}>` : mailbox.address}
              </option>
            ))}
          </NativeSelect>
        </div>
      )}
      <div className="grid gap-2">
        <AddressField
          label={t('composer.to')}
          value={to}
          suggestions={suggestions}
          autoFocus={!reply}
          onChange={(value) => {
            markDirty()
            setTo(value)
          }}
        />
        {submitted && recipientsMissing && <p className="text-xs text-destructive">{t('composer.recipientsRequired')}</p>}
        <RecipientSuggestions
          suggestions={suggestions}
          selected={[...to, ...cc, ...bcc]}
          onPick={(entry) => {
            markDirty()
            setTo((current) => [...current, entry])
          }}
        />
        {!showCopies && (
          <Button type="button" variant="link" size="sm" className="h-auto w-fit p-0" onClick={() => setShowCopies(true)}>
            {t('composer.addCopies')}
          </Button>
        )}
      </div>
      {showCopies && (
        <div className="grid gap-4 sm:grid-cols-2">
          <AddressField
            label={t('composer.cc')}
            value={cc}
            suggestions={suggestions}
            onChange={(value) => {
              markDirty()
              setCc(value)
            }}
          />
          <AddressField
            label={t('composer.bcc')}
            value={bcc}
            suggestions={suggestions}
            onChange={(value) => {
              markDirty()
              setBcc(value)
            }}
          />
        </div>
      )}
      {availableTemplates.length > 0 && (
        <div className="grid content-start gap-1.5">
          <Label htmlFor={`${formId}-template`}>{t('composer.template')}</Label>
          <NativeSelect
            id={`${formId}-template`}
            value={template?.id ?? ''}
            disabled={applyTemplate.isPending}
            onChange={(event) => {
              if (event.target.value) applyTemplate.mutate(event.target.value)
              else setTemplate(null)
            }}
          >
            <option value="">{t('composer.noTemplate')}</option>
            {availableTemplates.map((item) => (
              <option key={item.id} value={item.id}>
                {item.scope === 'personal' ? t('composer.personalTemplate', { name: item.name }) : item.name}
              </option>
            ))}
          </NativeSelect>
        </div>
      )}
      <div className="grid content-start gap-1.5">
        <Label htmlFor={`${formId}-subject`}>{t('fields.subject')}</Label>
        <Input placeholder={t('placeholders.subject')}
          id={`${formId}-subject`}
          ref={subjectRef}
          value={subject}
          maxLength={300}
          aria-invalid={(submitted && subject.trim() === '') || undefined}
          onFocus={() => {
            lastFocus.current = 'subject'
          }}
          onChange={(event) => {
            markDirty()
            setSubject(event.target.value)
          }}
        />
        {submitted && subject.trim() === '' && <p className="text-xs text-destructive">{t('composer.subjectRequired')}</p>}
      </div>
      <Tabs value={tab} onValueChange={changeTab} className="grid gap-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <TabsList>
            <TabsTrigger value="write">{t('composer.write')}</TabsTrigger>
            <TabsTrigger value="preview">{t('composer.preview')}</TabsTrigger>
          </TabsList>
          {tab === 'write' && (
            <div className="flex flex-wrap items-center gap-2">
              <NativeSelect
                aria-label={t('composer.format')}
                value={format}
                className="w-28"
                onChange={(event) => {
                  const next = event.target.value as BodyFormat
                  if (next !== 'text' && format === 'text') setBody(textToHtml(body))
                  if (next === 'text' && format !== 'text') setBody(htmlToText(body))
                  setFormat(next)
                }}
              >
                {BODY_FORMATS.map((option) => (
                  <option key={option} value={option}>
                    {t(`composer.formats.${option}`)}
                  </option>
                ))}
              </NativeSelect>
              <VariablePicker recordType={record.record_type} onInsert={insertVariable} />
            </div>
          )}
        </div>
        <TabsContent value="write" className="grid content-start gap-1.5">
          <Label htmlFor={`${formId}-body`} className="sr-only">
            {t('composer.body')}
          </Label>
          {format === 'rich' ? (
            <Suspense fallback={<Textarea rows={12} disabled placeholder={t('composer.editor.loading')} />}>
              <RichEmailEditor
                ref={richRef}
                id={`${formId}-body`}
                label={t('composer.body')}
                value={body}
                placeholder={t('composer.bodyPlaceholder')}
                onFocus={() => {
                  lastFocus.current = 'body'
                }}
                onChange={(html) => {
                  markDirty()
                  setBody(html)
                }}
              />
            </Suspense>
          ) : (
            <Textarea
              id={`${formId}-body`}
              ref={bodyRef}
              value={body}
              rows={12}
              className={format === 'html' ? 'font-mono text-xs' : undefined}
              placeholder={t('composer.bodyPlaceholder')}
              onFocus={() => {
                lastFocus.current = 'body'
              }}
              onChange={(event) => {
                markDirty()
                setBody(event.target.value)
              }}
            />
          )}
          {format === 'html' && <p className="text-xs text-muted-foreground">{t('composer.htmlHint')}</p>}
        </TabsContent>
        <TabsContent value="preview">
          <PreviewPane preview={preview.data} pending={preview.isPending} error={preview.isError} />
        </TabsContent>
      </Tabs>

      <div className="grid gap-2">
        <div className="flex flex-wrap items-center gap-2">
          <Button type="button" variant="outline" size="sm" onClick={() => fileInput.current?.click()}>
            <Paperclip aria-hidden="true" />
            {t('composer.attach')}
          </Button>
          <input
            ref={fileInput}
            type="file"
            multiple
            className="hidden"
            onChange={(event) => {
              addFiles(event.target.files)
              event.target.value = ''
            }}
          />
        </div>
        {attachments.length > 0 && (
          <ul className="grid gap-1">
            {attachments.map((attachment) => (
              <li
                key={attachment.key}
                className="flex items-center gap-2 rounded-md border px-2 py-1.5 text-sm"
              >
                <Paperclip className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
                <span className="min-w-0 flex-1 truncate">{attachment.name}</span>
                <span className="shrink-0 text-xs text-muted-foreground">
                  {attachment.failed
                    ? t('composer.uploadFailed')
                    : attachment.fileId
                      ? formatFileSize(attachment.size, formatting.locale)
                      : t('composer.uploading', { percent: Math.round(attachment.progress * 100) })}
                </span>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="size-7"
                  aria-label={t('composer.removeAttachment', { name: attachment.name })}
                  onClick={() => setAttachments((current) => current.filter((item) => item.key !== attachment.key))}
                >
                  <X />
                </Button>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="grid gap-3 rounded-md border p-3">
        <div className="flex items-start gap-3">
          <Checkbox
            id={`${formId}-schedule`}
            checked={scheduled}
            className="mt-0.5"
            onCheckedChange={(value) => {
              markDirty()
              setScheduled(value === true)
            }}
          />
          <div className="grid min-w-0 flex-1 gap-2">
            <Label htmlFor={`${formId}-schedule`} className="flex items-center gap-1.5">
              <CalendarClock className="size-4" aria-hidden="true" />
              {t('composer.schedule')}
            </Label>
            {scheduled && (
              <>
                <DateTimeFieldInput
                  aria-label={t('composer.scheduleAt')}
                  value={scheduledAt}
                  timeZone={formatting.timezone}
                  locale={formatting.locale}
                  invalid={submitted && scheduleInvalid}
                  onChange={(value) => {
                    setScheduleInvalid(false)
                    setScheduledAt(value)
                  }}
                />
                {submitted && scheduleInvalid && (
                  <p className="text-xs text-destructive">{t('composer.scheduleInvalid')}</p>
                )}
              </>
            )}
          </div>
        </div>
        {trackingAvailable && (
          <div className="flex items-start gap-3">
            <Checkbox
              id={`${formId}-tracking`}
              checked={trackingEnabled}
              className="mt-0.5"
              onCheckedChange={(value) => setTrackingOverride(value === true)}
            />
            <div className="grid gap-0.5">
              <Label htmlFor={`${formId}-tracking`}>{t('composer.tracking')}</Label>
              <p className="text-xs text-muted-foreground">{t('composer.trackingHint')}</p>
            </div>
          </div>
        )}
      </div>

      <DialogFooter>
        <Button type="button" variant="outline" onClick={onCancel} disabled={send.isPending}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" disabled={send.isPending || uploading}>
          {send.isPending ? (
            <Loader2 className="animate-spin" aria-hidden="true" />
          ) : scheduled ? (
            <CalendarClock aria-hidden="true" />
          ) : (
            <Send aria-hidden="true" />
          )}
          {scheduled ? t('composer.scheduleSubmit') : t('composer.send')}
        </Button>
      </DialogFooter>
    </form>
  )
}

interface RecipientSuggestionsProps {
  suggestions: EmailAddress[]
  selected: EmailAddress[]
  onPick: (entry: EmailAddress) => void
}

function RecipientSuggestions({ suggestions, selected, onPick }: RecipientSuggestionsProps) {
  const { t } = useTranslation('email')
  const taken = new Set(selected.map((entry) => entry.address.toLowerCase()))
  const options = suggestions.filter((entry) => !taken.has(entry.address.toLowerCase())).slice(0, 6)
  if (options.length === 0) return null
  return (
    <div className="flex flex-wrap items-center gap-1.5 text-xs">
      <span className="text-muted-foreground">{t('composer.suggestions')}</span>
      {options.map((entry) => (
        <button
          key={entry.address}
          type="button"
          className="max-w-full truncate rounded-full border px-2 py-0.5 hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
          title={entry.address}
          onClick={() => onPick(entry)}
        >
          {entry.name ? `${entry.name} · ${entry.address}` : entry.address}
        </button>
      ))}
    </div>
  )
}
