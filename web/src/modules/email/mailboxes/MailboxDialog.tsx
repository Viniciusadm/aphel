import { Loader2 } from 'lucide-react'
import { useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { UnsavedChangesDialog, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
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
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { emailApi, useInvalidateMailboxes } from '../api'
import { VisibilityPicker } from '../components/VisibilityPicker'
import { notifyEmailError } from '../lib/errors'
import {
  MAX_HISTORY_DAYS,
  emptyMailboxForm,
  imapSettings,
  isDefaultPort,
  isImapProvider,
  mailboxFormErrors,
  mailboxFormFrom,
  portFor,
  type MailboxFormValues,
} from '../lib/mailbox-form'
import { MAIL_SECURITIES, type MailSecurity, type Mailbox, type UpdateMailboxRequest } from '../types'
import { Field } from '@/components/ui/field'

export type MailboxDialogMode = { kind: 'create'; provider: string; providerLabel: string } | { kind: 'edit'; mailbox: Mailbox }

interface MailboxDialogProps {
  mode: MailboxDialogMode | null
  onOpenChange: (open: boolean) => void
}

export function MailboxDialog({ mode, onOpenChange }: MailboxDialogProps) {
  const { t } = useTranslation('email')
  const dirty = useRef(false)
  const [confirmingClose, setConfirmingClose] = useState(false)

  const requestClose = (next: boolean) => {
    if (next) return
    if (dirty.current) setConfirmingClose(true)
    else onOpenChange(false)
  }

  const title =
    mode?.kind === 'edit'
      ? t('mailboxes.editTitle')
      : t('mailboxes.connectTitle', { provider: mode?.providerLabel ?? '' })

  return (
    <>
      <Dialog open={mode !== null} onOpenChange={requestClose}>
        <DialogContent size="xl">
          <DialogHeader>
            <DialogTitle>{title}</DialogTitle>
            <DialogDescription>
              {mode?.kind === 'edit' ? mode.mailbox.address : t('mailboxes.connectDescription')}
            </DialogDescription>
          </DialogHeader>
          {mode && (
            <MailboxForm
              key={mode.kind === 'edit' ? mode.mailbox.id : mode.provider}
              mode={mode}
              onDirtyChange={(value) => {
                dirty.current = value
              }}
              onCancel={() => requestClose(false)}
              onDone={() => {
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

interface MailboxFormProps {
  mode: MailboxDialogMode
  onDirtyChange: (dirty: boolean) => void
  onCancel: () => void
  onDone: () => void
}

function MailboxForm({ mode, onDirtyChange, onCancel, onDone }: MailboxFormProps) {
  const { t } = useTranslation('email')
  const formId = useId()
  const invalidate = useInvalidateMailboxes()
  const editing = mode.kind === 'edit' ? mode.mailbox : null
  const provider = editing ? editing.provider : mode.kind === 'create' ? mode.provider : ''
  const withServer = isImapProvider(provider)
  const [initial] = useState<MailboxFormValues>(() => (editing ? mailboxFormFrom(editing) : emptyMailboxForm()))
  const [values, setValues] = useState(initial)
  const [submitted, setSubmitted] = useState(false)
  const [saving, setSaving] = useState(false)
  const errors = mailboxFormErrors(values, withServer, !editing)
  const hasErrors = Object.values(errors).some(Boolean)

  const update = (patch: Partial<MailboxFormValues>) => {
    setValues((current) => {
      const next = { ...current, ...patch }
      onDirtyChange(JSON.stringify(next) !== JSON.stringify(initial))
      return next
    })
  }

  const changeSecurity = (kind: 'imap' | 'smtp', security: MailSecurity) => {
    const portKey = kind === 'imap' ? 'imap_port' : 'smtp_port'
    const securityKey = kind === 'imap' ? 'imap_security' : 'smtp_security'
    const port = values[portKey]
    update({ [securityKey]: security, ...(isDefaultPort(kind, port) ? { [portKey]: portFor(kind, security) } : {}) })
  }

  const submit = async () => {
    setSubmitted(true)
    if (hasErrors) return
    setSaving(true)
    try {
      const displayName = values.display_name.trim() || null
      const historyDays = Number(values.history_days)
      if (editing) {
        const request: UpdateMailboxRequest = {
          display_name: displayName,
          default_visibility: values.default_visibility,
          history_days: historyDays,
        }
        if (withServer) request.settings = { ...imapSettings(values) }
        if (withServer && values.password) request.credentials = { password: values.password }
        await emailApi.updateMailbox(editing.id, request)
        notifySuccess(t('mailboxes.updated'))
      } else {
        await emailApi.createMailbox({
          provider,
          display_name: displayName,
          history_days: historyDays,
          default_visibility: values.default_visibility,
          settings: { ...imapSettings(values) },
          credentials: { password: values.password },
        })
        notifySuccess(t('mailboxes.connected'))
      }
      await invalidate()
      onDone()
    } catch (error) {
      notifyEmailError(error)
    } finally {
      setSaving(false)
    }
  }

  const invalid = (key: keyof typeof errors) => submitted && errors[key]
  const errorText = (key: keyof typeof errors, message: string) =>
    invalid(key) ? (
      <p id={`${formId}-${key}-error`} className="text-sm text-destructive">
        {message}
      </p>
    ) : null

  return (
    <form
      className="grid gap-5"
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <Field label={t('mailboxes.form.displayName')} className="content-start">
          {(control) => <Input
            {...control}
            value={values.display_name}
            maxLength={120}
            placeholder={t('mailboxes.form.displayNamePlaceholder')}
            onChange={(event) => update({ display_name: event.target.value })}
          />}
        </Field>
        <Field label={t('mailboxes.form.historyDays')} className="content-start">
          {(control) => <>
            <Input placeholder={t('placeholders.historyDays')}
            {...control}
            type="number"
            inputMode="numeric"
            min={1}
            max={MAX_HISTORY_DAYS}
            value={values.history_days}
            aria-invalid={invalid('history_days') || undefined}
            aria-describedby={`${formId}-history-hint`}
            onChange={(event) => update({ history_days: event.target.value })}
          />
          <p id={`${formId}-history-hint`} className="text-xs text-muted-foreground">
            {t('mailboxes.form.historyHint', { max: MAX_HISTORY_DAYS })}
          </p>
          {errorText('history_days', t('mailboxes.form.historyInvalid', { max: MAX_HISTORY_DAYS }))}
          </>}
        </Field>
      </div>

      {withServer && (
        <>
          <ServerFields
            kind="imap"
            formId={formId}
            values={values}
            submitted={submitted}
            errors={errors}
            onChange={update}
            onSecurityChange={(security) => changeSecurity('imap', security)}
          />
          <ServerFields
            kind="smtp"
            formId={formId}
            values={values}
            submitted={submitted}
            errors={errors}
            onChange={update}
            onSecurityChange={(security) => changeSecurity('smtp', security)}
          />
          <fieldset className="grid gap-4 sm:grid-cols-2">
            <legend className="mb-2 text-sm font-medium">{t('mailboxes.form.credentials')}</legend>
            <Field label={t('mailboxes.form.username')} className="content-start">
              {(control) => <>
                <Input placeholder={t('placeholders.username')}
                {...control}
                value={values.username}
                autoComplete="off"
                autoCapitalize="off"
                spellCheck={false}
                aria-invalid={invalid('username') || undefined}
                onChange={(event) => update({ username: event.target.value })}
              />
              {errorText('username', t('mailboxes.form.required'))}
              </>}
            </Field>
            <div className="grid content-start gap-1.5">
              <div className="flex items-center justify-between gap-2">
                <Label htmlFor={`${formId}-password`}>{t('mailboxes.form.password')}</Label>
                {editing?.credentials_configured && <Badge variant="secondary">{t('mailboxes.passwordConfigured')}</Badge>}
              </div>
              <Input
                id={`${formId}-password`}
                type="password"
                value={values.password}
                autoComplete="new-password"
                placeholder={editing ? t('mailboxes.form.passwordKeep') : undefined}
                aria-invalid={invalid('password') || undefined}
                onChange={(event) => update({ password: event.target.value })}
              />
              {errorText('password', t('mailboxes.form.required'))}
            </div>
          </fieldset>
        </>
      )}

      <fieldset className="grid gap-2">
        <legend className="mb-1 text-sm font-medium">{t('mailboxes.form.defaultVisibility')}</legend>
        <p className="text-xs text-muted-foreground">{t('mailboxes.form.visibilityHint')}</p>
        <VisibilityPicker
          name={`${formId}-visibility`}
          value={values.default_visibility}
          onChange={(visibility) => update({ default_visibility: visibility })}
        />
      </fieldset>

      <DialogFooter>
        <Button type="button" variant="outline" onClick={onCancel} disabled={saving}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" disabled={saving}>
          {saving && <Loader2 className="animate-spin" aria-hidden="true" />}
          {editing ? t('actions.save') : t('mailboxes.connect')}
        </Button>
      </DialogFooter>
    </form>
  )
}

interface ServerFieldsProps {
  kind: 'imap' | 'smtp'
  formId: string
  values: MailboxFormValues
  submitted: boolean
  errors: ReturnType<typeof mailboxFormErrors>
  onChange: (patch: Partial<MailboxFormValues>) => void
  onSecurityChange: (security: MailSecurity) => void
}

function ServerFields({ kind, formId, values, submitted, errors, onChange, onSecurityChange }: ServerFieldsProps) {
  const { t } = useTranslation('email')
  const hostKey = kind === 'imap' ? 'imap_host' : 'smtp_host'
  const portKey = kind === 'imap' ? 'imap_port' : 'smtp_port'
  const securityKey = kind === 'imap' ? 'imap_security' : 'smtp_security'
  const prefix = `${formId}-${kind}`
  return (
    <fieldset className="grid gap-4 sm:grid-cols-[minmax(0,1fr)_7rem_9rem]">
      <legend className="mb-2 text-sm font-medium">{t(`mailboxes.form.${kind}Legend`)}</legend>
      <Field label={t('mailboxes.form.host')} className="content-start">
        {(control) => <>
          <Input
          {...control}
          value={values[hostKey]}
          autoCapitalize="off"
          spellCheck={false}
          placeholder={kind === 'imap' ? 'imap.exemplo.com' : 'smtp.exemplo.com'}
          aria-invalid={(submitted && errors[hostKey]) || undefined}
          onChange={(event) => onChange({ [hostKey]: event.target.value })}
        />
        {submitted && errors[hostKey] && <p className="text-sm text-destructive">{t('mailboxes.form.required')}</p>}
        </>}
      </Field>
      <Field label={t('mailboxes.form.port')} className="content-start">
        {(control) => <>
          <Input placeholder={t('placeholders.port')}
          {...control}
          type="number"
          inputMode="numeric"
          min={1}
          max={65535}
          value={values[portKey]}
          aria-invalid={(submitted && errors[portKey]) || undefined}
          onChange={(event) => onChange({ [portKey]: event.target.value })}
        />
        {submitted && errors[portKey] && <p className="text-sm text-destructive">{t('mailboxes.form.portInvalid')}</p>}
        </>}
      </Field>
      <div className="grid content-start gap-2.5">
        <Label htmlFor={`${prefix}-security`}>{t('mailboxes.form.security')}</Label>
        <NativeSelect
          id={`${prefix}-security`}
          value={values[securityKey]}
          onChange={(event) => onSecurityChange(event.target.value as MailSecurity)}
        >
          {MAIL_SECURITIES.map((security) => (
            <option key={security} value={security}>
              {t(`mailboxes.security.${security}`)}
            </option>
          ))}
        </NativeSelect>
      </div>
    </fieldset>
  )
}
