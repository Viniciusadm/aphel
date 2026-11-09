import { useQuery } from '@tanstack/react-query'
import { useId, useState, type FormEvent, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '@/api'
import { notifyError, notifySuccess } from '@/components/feedback'
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
import { MaskedInput } from '@/components/ui/masked-input'
import { NumberMaskInput } from '@/components/ui/number-mask-input'
import { isTextMaskKind, isTextMaskValid } from '@/lib/text-mask'
import { NativeSelect } from '@/components/ui/native-select'
import { PLUGINS_PATH, type InstalledPlugin, type PluginSetting } from './types'
import { Field } from '@/components/ui/field'

interface EntityOptionsProps {
  id: string
  source: NonNullable<PluginSetting['options_from']>
  value: string
  placeholder: string
  onChange: (value: string) => void
  fallback: ReactNode
}

function EntityOptionsSelect({ id, source, value, placeholder, onChange, fallback }: EntityOptionsProps) {
  const records = useQuery({
    queryKey: ['plugin-setting-options', source.entity],
    queryFn: () =>
      api.get<{ data: Record<string, unknown>[] }>(`/v1/entities/${source.entity}/records`, { query: { per_page: '100' } }),
    retry: false,
  })
  if (records.isError) return <>{fallback}</>
  const options = (records.data?.data ?? [])
    .map((record) => ({ value: String(record[source.value] ?? ''), label: String(record[source.label] ?? record[source.value] ?? '') }))
    .filter((option) => option.value)
  const known = !value || options.some((option) => option.value === value)
  return (
    <NativeSelect id={id} value={value} disabled={records.isPending} onValueChange={onChange}>
      <option value="">{placeholder}</option>
      {!known && <option value={value}>{value}</option>}
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </NativeSelect>
  )
}

interface PluginSettingsDialogProps {
  plugin: InstalledPlugin
  onClose: () => void
  onSaved: () => Promise<unknown>
}

type Draft = Record<string, string | boolean>

function initialDraft(settings: PluginSetting[]): Draft {
  return Object.fromEntries(
    settings.map((setting) => {
      if (setting.type === 'boolean') return [setting.key, setting.value === true]
      if ((setting.type === 'secret' || (setting.type === 'tls_identity' || setting.type === 'signing_identity') || setting.type === 'http_credential') || setting.value === null || setting.value === undefined) return [setting.key, '']
      return [setting.key, String(setting.value)]
    }),
  )
}

function toPayload(settings: PluginSetting[], draft: Draft, cleared: string[]): Record<string, unknown> {
  const payload: Record<string, unknown> = {}
  for (const setting of settings) {
    const value = draft[setting.key]
    if (setting.type === 'secret' || (setting.type === 'tls_identity' || setting.type === 'signing_identity') || setting.type === 'http_credential') {
      if (cleared.includes(setting.key)) payload[setting.key] = null
      else if (typeof value === 'string' && value !== '') payload[setting.key] = value
    } else if (setting.type === 'boolean') {
      payload[setting.key] = value === true
    } else if (typeof value === 'string' && value.trim() === '') {
      payload[setting.key] = null
    } else if (setting.type === 'number') {
      payload[setting.key] = Number(value)
    } else {
      payload[setting.key] = value
    }
  }
  return payload
}

export function PluginSettingsDialog({ plugin, onClose, onSaved }: PluginSettingsDialogProps) {
  const { t } = useTranslation('modules')
  const { t: tForm } = useTranslation('records-form')
  const formId = useId()
  const [draft, setDraft] = useState<Draft>(() => initialDraft(plugin.settings))
  const [cleared, setCleared] = useState<string[]>([])
  const [saving, setSaving] = useState(false)
  const [invalid, setInvalid] = useState<Record<string, string>>({})

  const maskIssues = () => {
    const issues: Record<string, string> = {}
    for (const setting of plugin.settings) {
      const value = draft[setting.key]
      if (isTextMaskKind(setting.mask) && typeof value === 'string' && value.trim() !== '' && !isTextMaskValid(value, setting.mask)) {
        issues[setting.key] = tForm('validation.mask', { context: setting.mask })
      }
    }
    return issues
  }

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    const issues = maskIssues()
    setInvalid(issues)
    if (Object.keys(issues).length > 0) return
    setSaving(true)
    try {
      await api.put(`${PLUGINS_PATH}/${plugin.key}/settings`, toPayload(plugin.settings, draft, cleared))
      await onSaved()
      notifySuccess(t('plugins.settingsSaved'))
      onClose()
    } catch (error) {
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose()
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('plugins.settingsTitle', { name: plugin.name })}</DialogTitle>
          <DialogDescription>{t('plugins.settingsDescription')}</DialogDescription>
        </DialogHeader>
        <form id={formId} className="grid gap-4" onSubmit={(event) => void submit(event)}>
          {plugin.settings.map((setting, index) => {
            const heading =
              setting.group && setting.group !== plugin.settings[index - 1]?.group ? setting.group : null
            const inputId = `${formId}-${setting.key}`
            const value = draft[setting.key]
            const headingNode = heading && (
              <h3 key={`${setting.key}-heading`} className="mt-2 text-sm font-semibold">
                {heading}
              </h3>
            )
            if (setting.type === 'boolean') {
              return [
                headingNode,
                <div key={setting.key} className="flex items-start gap-2">
                  <Checkbox
                    id={inputId}
                    checked={value === true}
                    onCheckedChange={(checked) => setDraft({ ...draft, [setting.key]: checked === true })}
                  />
                  <Field id={inputId} label={setting.label}>
                    {setting.help && <p className="text-xs text-muted-foreground">{setting.help}</p>}
                  </Field>
                </div>,
              ]
            }
            if (setting.type === 'select') {
              return [
                headingNode,
                <Field key={setting.key} id={inputId} label={setting.label}>
                  <NativeSelect
                    id={inputId}
                    value={typeof value === 'string' ? value : ''}
                    onValueChange={(next) => setDraft({ ...draft, [setting.key]: next })}
                  >
                    <option value="">{t('placeholders.value')}</option>
                    {(setting.options ?? []).map((option) => (
                      <option key={option.value} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </NativeSelect>
                  {setting.help && <p className="text-xs text-muted-foreground">{setting.help}</p>}
                </Field>,
              ]
            }
            const isSecret = (setting.type === 'secret' || (setting.type === 'tls_identity' || setting.type === 'signing_identity') || setting.type === 'http_credential')
            const willClear = cleared.includes(setting.key)
            return [
              headingNode,
              <Field key={setting.key} id={inputId} label={setting.label} error={invalid[setting.key]}>
                {(setting.type === 'tls_identity' || setting.type === 'signing_identity') ? (
                  <Input
                    id={inputId}
                    type="file"
                    accept=".pem"
                    disabled={willClear || saving}
                    onChange={(event) => {
                      const file = event.target.files?.[0]
                      if (!file) return
                      if (file.size > 8192) {
                        event.target.value = ''
                        notifyError(new Error('TLS identity must have at most 8192 bytes'))
                        return
                      }
                      void file.text().then((pem) => setDraft((previous) => ({ ...previous, [setting.key]: pem }))).catch(notifyError)
                    }}
                  />
                ) : setting.options_from && setting.type === 'text' ? (
                  <EntityOptionsSelect
                    id={inputId}
                    source={setting.options_from}
                    value={typeof value === 'string' ? value : ''}
                    placeholder={t('placeholders.none')}
                    onChange={(next) => setDraft({ ...draft, [setting.key]: next })}
                    fallback={
                      <Input
                        placeholder={t('placeholders.value')}
                        id={inputId}
                        value={typeof value === 'string' ? value : ''}
                        onChange={(event) => setDraft({ ...draft, [setting.key]: event.target.value })}
                      />
                    }
                  />
                ) : setting.type === 'text' && isTextMaskKind(setting.mask) ? (
                  <MaskedInput
                    id={inputId}
                    mask={setting.mask}
                    aria-invalid={invalid[setting.key] ? true : undefined}
                    value={typeof value === 'string' ? value : ''}
                    onValueChange={(next) => setDraft({ ...draft, [setting.key]: next })}
                  />
                ) : setting.type === 'number' ? (
                  <NumberMaskInput
                    id={inputId}
                    kind={setting.min !== null && setting.min >= 0 ? 'integer' : 'signedInteger'}
                    placeholder={t('placeholders.value')}
                    value={typeof value === 'string' ? value : ''}
                    onValueChange={(next) => setDraft({ ...draft, [setting.key]: next })}
                  />
                ) : <Input placeholder={t('placeholders.value')}
                  id={inputId}
                  type={isSecret ? 'password' : 'text'}
                  autoComplete={isSecret ? 'new-password' : undefined}
                  disabled={willClear}
                  value={typeof value === 'string' ? value : ''}
                  onChange={(event) => setDraft({ ...draft, [setting.key]: event.target.value })}
                />}
                {setting.help && <p className="text-xs text-muted-foreground">{setting.help}</p>}
                {isSecret && (
                  <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                    <span>
                      {willClear
                        ? t('plugins.secretWillClear')
                        : setting.configured
                          ? t('plugins.secretConfigured')
                          : t('plugins.secretMissing')}
                    </span>
                    {setting.configured && !willClear && (
                      <Button
                        type="button"
                        size="sm"
                        variant="ghost"
                        onClick={() => setCleared([...cleared, setting.key])}
                      >
                        {t('plugins.secretClear')}
                      </Button>
                    )}
                  </div>
                )}
              </Field>,
            ]
          })}
        </form>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={onClose}>
            {t('plugins.cancel')}
          </Button>
          <Button type="submit" form={formId} disabled={saving}>
            {t('plugins.save')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
