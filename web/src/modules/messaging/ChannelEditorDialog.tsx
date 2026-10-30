import { Loader2 } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { isApiError } from '@/api'
import { notifyError } from '@/components/feedback'
import { UserFieldInput } from '@/components/fields'
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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { MESSAGING_KEY, messagingApi, type Channel, type ChannelBody, type ChannelKind } from './api'

interface FieldSpec {
  key: string
  type?: 'text' | 'number' | 'email'
  required?: boolean
}

const CONFIG_FIELDS: Record<ChannelKind, FieldSpec[]> = {
  whatsapp: [
    { key: 'phone_number_id', required: true },
    { key: 'business_account_id' },
    { key: 'display_phone_number' },
  ],
  email: [
    { key: 'address', type: 'email', required: true },
    { key: 'display_name' },
    { key: 'imap_host', required: true },
    { key: 'imap_port', type: 'number' },
    { key: 'imap_username' },
    { key: 'smtp_host', required: true },
    { key: 'smtp_port', type: 'number' },
    { key: 'smtp_username' },
  ],
}

const SECRET_FIELDS: Record<ChannelKind, { key: string; required: boolean }[]> = {
  whatsapp: [
    { key: 'access_token', required: true },
    { key: 'app_secret', required: true },
    { key: 'verify_token', required: true },
  ],
  email: [
    { key: 'imap_password', required: true },
    { key: 'smtp_password', required: false },
  ],
}

const DEFAULTS: Record<ChannelKind, Record<string, string>> = {
  whatsapp: {},
  email: { imap_port: '993', smtp_port: '465', smtp_security: 'tls' },
}

interface Editing {
  kind: ChannelKind
  channel: Channel | null
}

interface ChannelEditorDialogProps {
  editing: Editing | null
  onOpenChange: (open: boolean) => void
  onSaved: (channel: Channel, created: boolean) => void
}

export function ChannelEditorDialog({ editing, onOpenChange, onSaved }: ChannelEditorDialogProps) {
  const t = useModuleTranslation(MESSAGING_KEY)
  return (
    <Dialog open={editing !== null} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        {editing && (
          <>
            <DialogHeader>
              <DialogTitle>
                {editing.channel
                  ? t('editor.editTitle', { name: editing.channel.name })
                  : t(`editor.newTitle.${editing.kind}`)}
              </DialogTitle>
              <DialogDescription>{t(`editor.help.${editing.kind}`)}</DialogDescription>
            </DialogHeader>
            <ChannelForm
              key={editing.channel?.id ?? editing.kind}
              editing={editing}
              onCancel={() => onOpenChange(false)}
              onSaved={onSaved}
            />
          </>
        )}
      </DialogContent>
    </Dialog>
  )
}

function initialConfig(editing: Editing): Record<string, string> {
  const values: Record<string, string> = { ...DEFAULTS[editing.kind] }
  for (const [key, value] of Object.entries(editing.channel?.config ?? {})) {
    if (value !== null && value !== undefined) values[key] = String(value)
  }
  return values
}

function fieldErrors(error: unknown): Record<string, string> {
  if (!isApiError(error) || typeof error.details !== 'object' || error.details === null) return {}
  const fields = (error.details as { fields?: { field: string; message: string }[] }).fields ?? []
  return Object.fromEntries(fields.map((item) => [item.field, item.message]))
}

function Field({
  id,
  label,
  hint,
  error,
  children,
}: {
  id: string
  label: string
  hint?: string
  error?: string
  children: ReactNode
}) {
  return (
    <div className="grid gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      {children}
      {hint && !error && <p className="text-xs text-muted-foreground">{hint}</p>}
      {error && (
        <p id={`${id}-error`} className="text-xs text-destructive">
          {error}
        </p>
      )}
    </div>
  )
}

function ChannelForm({
  editing,
  onCancel,
  onSaved,
}: {
  editing: Editing
  onCancel: () => void
  onSaved: (channel: Channel, created: boolean) => void
}) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const { kind, channel } = editing
  const [name, setName] = useState(channel?.name ?? '')
  const [isActive, setIsActive] = useState(channel?.is_active ?? true)
  const [actingUserId, setActingUserId] = useState<string | null>(channel?.acting_user_id ?? null)
  const [config, setConfig] = useState<Record<string, string>>(() => initialConfig(editing))
  const [secrets, setSecrets] = useState<Record<string, string>>({})
  const [clearSmtp, setClearSmtp] = useState(false)
  const [errors, setErrors] = useState<Record<string, string>>({})
  const [saving, setSaving] = useState(false)

  const save = async () => {
    setSaving(true)
    setErrors({})
    const body: ChannelBody = {
      kind,
      name,
      is_active: isActive,
      acting_user_id: actingUserId,
      config: Object.fromEntries(
        Object.entries(config).map(([key, value]) => [
          key,
          CONFIG_FIELDS[kind].find((field) => field.key === key)?.type === 'number' && value.trim()
            ? Number(value)
            : value.trim() || null,
        ]),
      ),
      secrets: Object.fromEntries(Object.entries(secrets).filter(([, value]) => value.trim() !== '')),
      clear_secrets: clearSmtp ? ['smtp_password'] : [],
    }
    try {
      const saved = channel
        ? await messagingApi.updateChannel(channel.id, body)
        : await messagingApi.createChannel(body)
      onSaved(saved.data, channel === null)
    } catch (error) {
      const found = fieldErrors(error)
      setErrors(found)
      if (Object.keys(found).length === 0) notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  const prefix = channel?.id ?? `new-${kind}`

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        void save()
      }}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <Field id={`${prefix}-name`} label={t('editor.name')} error={errors.name}>
          <Input
            id={`${prefix}-name`}
            value={name}
            maxLength={120}
            required
            onChange={(event) => setName(event.target.value)}
            aria-invalid={errors.name ? true : undefined}
          />
        </Field>
        <Field
          id={`${prefix}-acting-user`}
          label={t('editor.actingUser')}
          hint={t('editor.actingUserHint')}
          error={errors.acting_user_id}
        >
          <UserFieldInput id={`${prefix}-acting-user`} value={actingUserId} onChange={setActingUserId} />
        </Field>
      </div>

      <fieldset className="grid gap-4 sm:grid-cols-2">
        <legend className="mb-2 text-sm font-semibold">{t('editor.connection')}</legend>
        {CONFIG_FIELDS[kind].map((field) => (
          <Field
            key={field.key}
            id={`${prefix}-${field.key}`}
            label={t(`editor.config.${field.key}`)}
            hint={t(`editor.hints.${field.key}`, { defaultValue: '' }) || undefined}
            error={errors[`config.${field.key}`]}
          >
            <Input
              id={`${prefix}-${field.key}`}
              type={field.type ?? 'text'}
              inputMode={field.type === 'number' ? 'numeric' : undefined}
              value={config[field.key] ?? ''}
              required={field.required}
              onChange={(event) => setConfig((current) => ({ ...current, [field.key]: event.target.value }))}
              aria-invalid={errors[`config.${field.key}`] ? true : undefined}
            />
          </Field>
        ))}
        {kind === 'email' && (
          <Field
            id={`${prefix}-smtp-security`}
            label={t('editor.config.smtp_security')}
            error={errors['config.smtp_security']}
          >
            <Select
              value={config.smtp_security ?? 'tls'}
              onValueChange={(value) => setConfig((current) => ({ ...current, smtp_security: value }))}
            >
              <SelectTrigger id={`${prefix}-smtp-security`}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="tls">{t('editor.security.tls')}</SelectItem>
                <SelectItem value="starttls">{t('editor.security.starttls')}</SelectItem>
              </SelectContent>
            </Select>
          </Field>
        )}
      </fieldset>

      <fieldset className="grid gap-4 sm:grid-cols-2">
        <legend className="mb-1 text-sm font-semibold">{t('editor.credentials')}</legend>
        <p className="text-xs text-muted-foreground sm:col-span-2">{t('editor.credentialsHelp')}</p>
        {SECRET_FIELDS[kind].map((field) => {
          const configured = channel?.secrets?.[field.key] === true
          return (
            <Field
              key={field.key}
              id={`${prefix}-${field.key}`}
              label={t(`editor.secrets.${field.key}`)}
              hint={configured ? t('editor.secretConfigured') : t(`editor.hints.${field.key}`, { defaultValue: '' }) || undefined}
              error={errors[`secrets.${field.key}`]}
            >
              <Input
                id={`${prefix}-${field.key}`}
                type="password"
                autoComplete="new-password"
                value={secrets[field.key] ?? ''}
                required={field.required && !configured}
                placeholder={configured ? t('editor.keepSecret') : undefined}
                onChange={(event) => setSecrets((current) => ({ ...current, [field.key]: event.target.value }))}
                aria-invalid={errors[`secrets.${field.key}`] ? true : undefined}
              />
            </Field>
          )
        })}
        {kind === 'email' && channel?.secrets?.smtp_password && (
          <label className="flex items-center gap-2 text-sm sm:col-span-2">
            <Checkbox checked={clearSmtp} onCheckedChange={(checked) => setClearSmtp(checked === true)} />
            {t('editor.reuseImapPassword')}
          </label>
        )}
      </fieldset>

      <label className="flex items-center gap-2 text-sm">
        <Checkbox checked={isActive} onCheckedChange={(checked) => setIsActive(checked === true)} />
        {t('editor.active')}
      </label>

      <DialogFooter>
        <Button type="button" variant="outline" onClick={onCancel}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" disabled={saving}>
          {saving && <Loader2 className="animate-spin" aria-hidden="true" />}
          {t('actions.save')}
        </Button>
      </DialogFooter>
    </form>
  )
}
