import { RefreshCw } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { toast } from '@/components/ui/toast-store'
import { formatDateTime, NativeSelect, SETTINGS_MANAGE, useRecordFormatting } from '@/sdk'
import {currenciesApi, useInvalidateCurrencies} from '@/sdk'
import {currencyErrorKey} from '@/sdk'
import type {CurrencySettings, RatesProviderKey} from '@/sdk'
import { Field } from '@/components/ui/field'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'

const PROVIDERS: RatesProviderKey[] = ['manual', 'frankfurter']
const URL_PATTERN = /^https?:\/\/\S+$/i

export function RatesProviderSettings({ settings }: { settings: CurrencySettings }) {
  const { t } = useTranslation('money')
  const { hasPermission } = useSession()
  const canEdit = hasPermission(SETTINGS_MANAGE)
  const formatting = useRecordFormatting()
  const invalidate = useInvalidateCurrencies()
  const urlId = useId()
  const [provider, setProvider] = useState<RatesProviderKey>(settings.provider.provider === 'frankfurter' ? 'frankfurter' : 'manual')
  const [url, setUrl] = useState(settings.provider.url ?? '')
  const [invalidUrl, setInvalidUrl] = useState(false)
  const [saving, setSaving] = useState(false)
  const [syncing, setSyncing] = useState(false)
  const dirty = provider !== settings.provider.provider || url.trim() !== (settings.provider.url ?? '')
  const configured = settings.provider.provider !== 'manual'

  const save = async () => {
    const trimmed = url.trim()
    if (trimmed && !URL_PATTERN.test(trimmed)) {
      setInvalidUrl(true)
      return
    }
    setSaving(true)
    try {
      await currenciesApi.updateProvider({ provider, url: trimmed || null })
      await invalidate()
      toast({ title: t('provider.saved'), variant: 'success' })
    } catch (error) {
      toast({ title: t(currencyErrorKey(error)), variant: 'destructive' })
    } finally {
      setSaving(false)
    }
  }

  const sync = async () => {
    setSyncing(true)
    try {
      const outcome = await currenciesApi.syncRates()
      await invalidate()
      toast({
        title: t('provider.synced'),
        description: t('provider.syncedSummary', {
          created: outcome.created.length,
          updated: outcome.updated.length,
          unchanged: outcome.unchanged.length + outcome.kept_manual.length,
        }),
        variant: 'success',
      })
      if (outcome.failed.length > 0) {
        toast({ title: t('provider.syncFailed', { currencies: outcome.failed.join(', ') }), variant: 'destructive' })
      }
    } catch (error) {
      toast({ title: t(currencyErrorKey(error)), variant: 'destructive' })
    } finally {
      setSyncing(false)
    }
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div className="grid gap-1.5">
            <CardTitle>{t('provider.title')}</CardTitle>
            <CardDescription>{t('provider.description')}</CardDescription>
          </div>
          {canEdit && (
            <div className="flex flex-wrap gap-2">
              {configured && (
                <Button type="button" variant="outline" disabled={syncing || dirty} onClick={() => void sync()}>
                  <RefreshCw aria-hidden="true" className={syncing ? 'animate-spin' : undefined} />
                  {t('provider.syncNow')}
                </Button>
              )}
              <Button type="submit" form="rates-provider-form" disabled={saving || !dirty}>
                {t('provider.save')}
              </Button>
            </div>
          )}
        </div>
      </CardHeader>
      <CardContent className="grid gap-3">
        <form
          id="rates-provider-form"
          className="flex flex-wrap items-end gap-3"
          noValidate
          onSubmit={(event) => {
            event.preventDefault()
            void save()
          }}
        >
          <Field label={t('provider.label')}>
            {(control) => (
              <NativeSelect
                {...control}
                value={provider}
                disabled={!canEdit || saving}
                onChange={(event) => setProvider(event.target.value as RatesProviderKey)}
              >
                {PROVIDERS.map((key) => (
                  <option key={key} value={key}>
                    {key === 'manual' ? t('provider.options.manual') : t('provider.options.frankfurter')}
                  </option>
                ))}
              </NativeSelect>
            )}
          </Field>
          {provider !== 'manual' && (
            <Field label={t('provider.urlLabel')} className="min-w-64 flex-1">
              {(control) => (
                <Input
                  {...control}
                  type="url"
                  value={url}
                  placeholder="https://api.frankfurter.app"
                  disabled={!canEdit || saving}
                  aria-invalid={invalidUrl ? true : undefined}
                  aria-describedby={`${urlId}-hint`}
                  onChange={(event) => {
                    setUrl(event.target.value)
                    setInvalidUrl(false)
                  }}
                />
              )}
            </Field>
          )}
          {provider !== 'manual' && (
            <p id={`${urlId}-hint`} className={`basis-full text-sm ${invalidUrl ? 'text-destructive' : 'text-muted-foreground'}`}>
              {invalidUrl ? t('provider.invalidUrl') : t('provider.urlHint')}
            </p>
          )}
        </form>
        {configured && (
          <p className="text-sm text-muted-foreground">
            {settings.provider.last_synced_at
              ? t('provider.lastSynced', { date: formatDateTime(settings.provider.last_synced_at, formatting) })
              : t('provider.neverSynced')}
          </p>
        )}
      </CardContent>
    </Card>
  )
}
