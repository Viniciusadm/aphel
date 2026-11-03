import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect } from '@/components/ui/native-select'
import { Loader2 } from 'lucide-react'
import { useState } from 'react'
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
          <Input placeholder={t('editor.namePlaceholder')}
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
          description={t('editor.actingUserHint')}
          error={errors.acting_user_id}
        >
          <UserFieldInput id={`${prefix}-acting-user`} value={actingUserId} onChange={setActingUserId} />
        </Field>
      </div>

      <fieldset className="grid gap-4 sm:grid-cols-2">
        <legend className="mb-2 text-sm font-semibold">{t('editor.connection')}</legend>
        {provider.fields.map((field) => (
          <Field
            key={field.key}
            id={`${prefix}-${field.key}`}
            label={channelText(field.label)}
            description={channelText(field.hint) || undefined}
            error={errors[`config.${field.key}`]}
          >
            {field.input_type === 'select' ? (
              <NativeSelect id={`${prefix}-${field.key}`} value={config[field.key] ?? ''} onValueChange={(value) => setConfig((current) => ({ ...current, [field.key]: value }))}>
                {field.options.map((option) => <option key={option.value} value={option.value}>{channelText(option.label)}</option>)}
              </NativeSelect>
            ) : (
            <Input placeholder={t('editor.valuePlaceholder')}
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
      </fieldset>

      <fieldset className="grid gap-4 sm:grid-cols-2">
        <legend className="mb-1 text-sm font-semibold">{t('editor.credentials')}</legend>
        <p className="text-xs text-muted-foreground sm:col-span-2">{t('editor.credentialsHelp')}</p>
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
                placeholder={configured ? t('editor.keepSecret') : undefined}
                onChange={(event) => setSecrets((current) => ({ ...current, [field.key]: event.target.value }))}
                aria-invalid={errors[`secrets.${field.key}`] ? true : undefined}
              />
            </Field>
          )
        })}
        {provider.secrets.filter((field) => !field.required && channel?.secrets?.[field.key]).map((field) => (
          <label key={field.key} className="flex items-center gap-2 text-sm sm:col-span-2">
            <Checkbox checked={clearSecrets.includes(field.key)} onCheckedChange={(checked) => setClearSecrets((current) => checked === true ? [...current, field.key] : current.filter((key) => key !== field.key))} />
            {t('editor.clearCredential', { name: channelText(field.label) })}
          </label>
        ))}
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
