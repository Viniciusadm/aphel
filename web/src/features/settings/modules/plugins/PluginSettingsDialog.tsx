import { useId, useState, type FormEvent } from 'react'
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
import { Label } from '@/components/ui/label'
import { PLUGINS_PATH, type InstalledPlugin, type PluginSetting } from './types'

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
      if (setting.type === 'secret' || setting.value === null || setting.value === undefined) return [setting.key, '']
      return [setting.key, String(setting.value)]
    }),
  )
}

function toPayload(settings: PluginSetting[], draft: Draft, cleared: string[]): Record<string, unknown> {
  const payload: Record<string, unknown> = {}
  for (const setting of settings) {
    const value = draft[setting.key]
    if (setting.type === 'secret') {
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
  const formId = useId()
  const [draft, setDraft] = useState<Draft>(() => initialDraft(plugin.settings))
  const [cleared, setCleared] = useState<string[]>([])
  const [saving, setSaving] = useState(false)

  const submit = async (event: FormEvent) => {
    event.preventDefault()
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
          {plugin.settings.map((setting) => {
            const inputId = `${formId}-${setting.key}`
            const value = draft[setting.key]
            if (setting.type === 'boolean') {
              return (
                <div key={setting.key} className="flex items-start gap-2">
                  <Checkbox
                    id={inputId}
                    checked={value === true}
                    onCheckedChange={(checked) => setDraft({ ...draft, [setting.key]: checked === true })}
                  />
                  <div className="grid gap-1">
                    <Label htmlFor={inputId}>{setting.label}</Label>
                    {setting.help && <p className="text-xs text-muted-foreground">{setting.help}</p>}
                  </div>
                </div>
              )
            }
            const isSecret = setting.type === 'secret'
            const willClear = cleared.includes(setting.key)
            return (
              <div key={setting.key} className="grid gap-1.5">
                <Label htmlFor={inputId}>{setting.label}</Label>
                <Input placeholder={t('placeholders.value')}
                  id={inputId}
                  type={isSecret ? 'password' : setting.type === 'number' ? 'number' : 'text'}
                  autoComplete={isSecret ? 'new-password' : undefined}
                  min={setting.min ?? undefined}
                  max={setting.max ?? undefined}
                  disabled={willClear}
                  value={typeof value === 'string' ? value : ''}
                  onChange={(event) => setDraft({ ...draft, [setting.key]: event.target.value })}
                />
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
              </div>
            )
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
