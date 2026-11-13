import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect } from '@/components/ui/native-select'
import { ExternalLink, Loader2 } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { isApiError } from '@/api'
import { notifyError } from '@/components/feedback'
import { UserFieldInput } from '@/components/fields'
import { Button } from '@/components/ui/button'
import { ToggleField } from '@/components/ui/toggle-field'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { MaskedInput } from '@/components/ui/masked-input'
import { PhoneFieldInput } from '@/components/fields/LazyPhoneFieldInput'
import { useRecordFormatting } from '@/sdk'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { MESSAGING_KEY, messagingApi, type Channel, type ChannelBody, type ChannelKind, type ChannelDefinition, useChannelProviders, channelText } from './api'
import { Field } from '@/components/ui/field'

interface Editing {
  kind: ChannelKind
  channel: Channel | null
}

interface ChannelEditorDialogProps {
  editing: Editing | null
  onOpenChange: (open: boolean) => void
  onSaved: (channel: Channel, created: boolean) => void
}

const GUIDE_LINKS = [
  'https://developers.facebook.com/apps/',
  'https://business.facebook.com/settings/system-users',
  'https://developers.facebook.com/docs/whatsapp/cloud-api/get-started',
]

export function ChannelEditorDialog({ editing, onOpenChange, onSaved }: ChannelEditorDialogProps) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const providers = useChannelProviders(editing !== null)
  const provider = providers.data?.find((item) => item.key === editing?.kind)
  return (
    <Dialog open={editing !== null} onOpenChange={onOpenChange}>
      <DialogContent size="xl">
        {editing && !provider && (providers.isPending ? <Skeleton className="h-40" /> : <ErrorState title={t('editor.providerUnavailable')} onRetry={() => void providers.refetch()} />)}
        {editing && provider && (
          <>
            <DialogHeader>
              <DialogTitle>
                {editing.channel
                  ? t('editor.editTitle', { name: editing.channel.name })
                  : t('editor.newChannel', { name: channelText(provider.label) })}
              </DialogTitle>
              <DialogDescription>{t('editor.providerHelp', { name: channelText(provider.label) })}</DialogDescription>
            </DialogHeader>
            <ChannelForm
              key={editing.channel?.id ?? editing.kind}
              editing={editing}
              provider={provider}
              onCancel={() => onOpenChange(false)}
              onSaved={onSaved}
            />
          </>
        )}
      </DialogContent>
    </Dialog>
  )
}

function initialConfig(editing: Editing, provider: ChannelDefinition): Record<string, string> {
  const values: Record<string, string> = { ...provider.defaults }
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

function ChannelForm({
  editing,
  provider,
  onCancel,
  onSaved,
}: {
  editing: Editing
  provider: ChannelDefinition
  onCancel: () => void
  onSaved: (channel: Channel, created: boolean) => void
}) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const { kind, channel } = editing
  const [name, setName] = useState(channel?.name ?? '')
  const [isActive, setIsActive] = useState(channel?.is_active ?? true)
  const [actingUserId, setActingUserId] = useState<string | null>(channel?.acting_user_id ?? null)
  const [config, setConfig] = useState<Record<string, string>>(() => initialConfig(editing, provider))
  const formatting = useRecordFormatting()
  const [secrets, setSecrets] = useState<Record<string, string>>({})
  const [clearSecrets, setClearSecrets] = useState<string[]>([])
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
          provider.fields.find((field) => field.key === key)?.input_type === 'number' && value.trim()
            ? Number(value)
            : value.trim() || null,
        ]),
      ),
      secrets: Object.fromEntries(Object.entries(secrets).filter(([, value]) => value.trim() !== '')),
      clear_secrets: clearSecrets,
    }
    try {
      const saved = channel
        ? await messagingApi.updateChannel(channel.id, body)
        : await messagingApi.createChannel(body)
      onSaved(saved, channel === null)
    } catch (error) {
      const found = fieldErrors(error)
      setErrors(found)
      if (Object.keys(found).length === 0) notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  const prefix = channel?.id ?? `new-${kind}`
  const guide = [0, 1, 2, 3, 4, 5, 6, 7].map((index) => t(`editor.guide.${kind}.${index}` as 'editor.guide.whatsapp.0', { defaultValue: '' })).filter(Boolean)
  const guideLinks = kind === 'whatsapp'
    ? GUIDE_LINKS.map((url, index) => ({ url, label: t(`editor.guideLinks.whatsapp.${index}` as 'editor.guideLinks.whatsapp.0', { defaultValue: '' }) })).filter((link) => link.label)
    : []
  const placeholderOf = (key: string) => t(`editor.placeholders.${key}` as 'editor.placeholders.phone_number_id', { defaultValue: '' }) || undefined

  return (
    <form
      className="grid gap-5"
      onSubmit={(event) => {
        event.preventDefault()
        void save()
      }}
    >
      {guide.length > 0 && !channel && (
        <div className="grid gap-2 rounded-lg border border-primary/20 bg-primary/5 p-4 text-sm">
          <p className="font-medium">{t(`editor.guideTitle.${kind}` as 'editor.guideTitle.whatsapp')}</p>
          <ol className="grid list-decimal gap-1 pl-5 text-muted-foreground">
            {guide.map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ol>
          {guideLinks.length > 0 && (
            <p className="flex flex-wrap gap-x-4 gap-y-1">
              {guideLinks.map((link) => (
                <a key={link.url} href={link.url} target="_blank" rel="noreferrer noopener" className="inline-flex items-center gap-1 font-medium text-primary underline-offset-4 hover:underline">
                  {link.label}
                  <ExternalLink className="size-3" aria-hidden="true" />
                </a>
              ))}
            </p>
          )}
        </div>
      )}

      <FormSection title={t('editor.sections.identity')} description={t('editor.sections.identityHelp')}>
        <Field id={`${prefix}-name`} label={t('editor.name')} error={errors.name}>
          <Input
            placeholder={t('editor.namePlaceholder')}
            id={`${prefix}-name`}
            value={name}
            maxLength={120}
            required
            onChange={(event) => setName(event.target.value)}
            aria-invalid={errors.name ? true : undefined}
          />
        </Field>
      </FormSection>

      <FormSection title={t('editor.connection')} description={t('editor.sections.connectionHelp')}>
        <div className="grid gap-4 sm:grid-cols-2 sm:items-start">
          {provider.fields.map((field) => (
            <Field
              key={field.key}
              id={`${prefix}-${field.key}`}
              label={channelText(field.label)}
              description={channelText(field.hint) || undefined}
              error={errors[`config.${field.key}`]}
              className={field.input_type === 'tel' ? 'sm:col-span-2' : undefined}
            >
              {field.input_type === 'select' ? (
                <NativeSelect id={`${prefix}-${field.key}`} value={config[field.key] ?? ''} onValueChange={(value) => setConfig((current) => ({ ...current, [field.key]: value }))}>
                  {field.options.map((option) => <option key={option.value} value={option.value}>{channelText(option.label)}</option>)}
                </NativeSelect>
              ) : field.input_type === 'tel' ? (
                <PhoneFieldInput
                  id={`${prefix}-${field.key}`}
                  locale={formatting.locale}
                  fieldLabel={channelText(field.label)}
                  invalid={Boolean(errors[`config.${field.key}`])}
                  value={config[field.key] || null}
                  onChange={(value) => setConfig((current) => ({ ...current, [field.key]: value ?? '' }))}
                />
              ) : field.input_type === 'digits' ? (
                <MaskedInput
                  id={`${prefix}-${field.key}`}
                  mask="digits"
                  maxLength={24}
                  placeholder={placeholderOf(field.key)}
                  value={config[field.key] ?? ''}
                  required={field.required}
                  onValueChange={(value) => setConfig((current) => ({ ...current, [field.key]: value }))}
                  aria-invalid={errors[`config.${field.key}`] ? true : undefined}
                />
              ) : (
                <Input
                  placeholder={placeholderOf(field.key) ?? t('editor.valuePlaceholder')}
                  id={`${prefix}-${field.key}`}
                  type={field.input_type}
                  inputMode={field.input_type === 'number' ? 'numeric' : undefined}
                  value={config[field.key] ?? ''}
                  required={field.required}
                  onChange={(event) => setConfig((current) => ({ ...current, [field.key]: event.target.value }))}
                  aria-invalid={errors[`config.${field.key}`] ? true : undefined}
                />
              )}
            </Field>
          ))}
        </div>
      </FormSection>

      <FormSection title={t('editor.credentials')} description={t(channel ? 'editor.credentialsHelp' : 'editor.credentialsHelpNew')}>
        <div className="grid gap-4">
          {provider.secrets.map((field) => {
            const configured = channel?.secrets?.[field.key] === true
            return (
              <Field
                key={field.key}
                id={`${prefix}-${field.key}`}
                label={channelText(field.label)}
                description={configured ? t('editor.secretConfigured') : channelText(field.hint) || undefined}
                error={errors[`secrets.${field.key}`]}
              >
                <Input
                  id={`${prefix}-${field.key}`}
                  type="password"
                  autoComplete="new-password"
                  value={secrets[field.key] ?? ''}
                  required={field.required && !configured}
                  placeholder={configured ? t('editor.keepSecret') : placeholderOf(field.key)}
                  onChange={(event) => setSecrets((current) => ({ ...current, [field.key]: event.target.value }))}
                  aria-invalid={errors[`secrets.${field.key}`] ? true : undefined}
                />
              </Field>
            )
          })}
          {provider.secrets.filter((field) => !field.required && channel?.secrets?.[field.key]).map((field) => (
            <ToggleField
              key={field.key}
              label={t('editor.clearCredential', { name: channelText(field.label) })}
              checked={clearSecrets.includes(field.key)}
              onCheckedChange={(checked) => setClearSecrets((current) => (checked ? [...current, field.key] : current.filter((key) => key !== field.key)))}
            />
          ))}
        </div>
      </FormSection>

      <FormSection title={t('editor.sections.options')}>
        <ToggleField type="switch" label={t('editor.active')} description={t('editor.activeHelp')} checked={isActive} onCheckedChange={setIsActive} />
        <details className="group rounded-md border px-3 py-2 text-sm" open={Boolean(actingUserId)}>
          <summary className="cursor-pointer font-medium">{t('editor.sections.advanced')}</summary>
          <div className="mt-3">
            <Field
              id={`${prefix}-acting-user`}
              label={t('editor.actingUser')}
              description={t('editor.actingUserHint')}
              error={errors.acting_user_id}
            >
              <UserFieldInput id={`${prefix}-acting-user`} value={actingUserId} onChange={setActingUserId} />
            </Field>
          </div>
        </details>
      </FormSection>

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

function FormSection({ title, description, children }: { title: string; description?: string; children: ReactNode }) {
  return (
    <section className="grid gap-3 rounded-lg border p-4">
      <div className="grid gap-0.5">
        <h3 className="text-sm font-semibold">{title}</h3>
        {description && <p className="text-xs text-muted-foreground">{description}</p>}
      </div>
      {children}
    </section>
  )
}
