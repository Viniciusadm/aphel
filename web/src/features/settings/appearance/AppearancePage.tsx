import { useId } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { api, type components } from '@/api'
import { useSession } from '@/app/session/session-context'
import { notifyError } from '@/components/feedback'
import { ErrorState } from '@/components/ui/error-state'
import { Field } from '@/components/ui/field'
import { NativeSelect } from '@/components/ui/native-select'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Skeleton } from '@/components/ui/skeleton'
import { usePreferences, useUpdatePreferences } from '@/features/command-palette/preferences/api'
import { CORE_UI } from '@/kernel/modules/ui-plugin-context'
import { SETTINGS_MANAGE } from '../lib/permissions'

type UiPlugin = components['schemas']['UiPluginResponse']

const FOLLOW_ORGANIZATION = ''

function Swatches({ colors }: { colors: Record<string, string> }) {
  const values = Object.values(colors)
  if (values.length === 0) return null
  return (
    <span className="flex gap-1" aria-hidden>
      {values.map((color, index) => (
        <span key={index} className="size-4 rounded-full border" style={{ background: color }} />
      ))}
    </span>
  )
}

export function AppearancePage() {
  const { t } = useTranslation('appearance')
  const { hasPermission } = useSession()
  const queryClient = useQueryClient()
  const groupId = useId()
  const defaultId = useId()
  const preferences = usePreferences()
  const plugins = useQuery({ queryKey: ['ui-plugins'], queryFn: () => api.get<UiPlugin[]>('/v1/ui-plugins') })
  const update = useUpdatePreferences()
  const setDefault = useMutation({
    mutationFn: (key: string | null) => api.put('/v1/organization/ui-plugin', { ui_plugin: key }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['preferences'] })
      window.location.reload()
    },
    onError: (error) => notifyError(error, { title: t('saveError') }),
  })

  const choose = (value: string) => {
    update.mutate(
      { ui_plugin: value === FOLLOW_ORGANIZATION ? null : value },
      { onSuccess: () => window.location.reload(), onError: (error) => notifyError(error, { title: t('saveError') }) },
    )
  }

  if (preferences.isPending || plugins.isPending) return <Skeleton className="h-64" />
  if (preferences.isError || plugins.isError) {
    return <ErrorState onRetry={() => { void preferences.refetch(); void plugins.refetch() }} />
  }

  const stored = preferences.data.ui_plugin ?? FOLLOW_ORGANIZATION
  const defaultKey = preferences.data.ui_plugin_default
  const known = new Map(plugins.data.map((plugin) => [plugin.key, plugin]))
  const defaultLabel = defaultKey ? known.get(defaultKey)?.label ?? defaultKey : t('coreName')
  const options: Array<{ value: string; label: string; description: string; plugin?: UiPlugin }> = [
    { value: FOLLOW_ORGANIZATION, label: t('followOrganization'), description: t('followOrganizationHint', { name: defaultLabel }) },
    { value: CORE_UI, label: t('coreName'), description: t('coreHint') },
    ...plugins.data.map((plugin) => ({ value: plugin.key, label: plugin.label, description: plugin.description ?? '', plugin })),
  ]

  return (
    <Page>
      <PageHeader title={t('title')} description={t('description')} />
      <fieldset disabled={update.isPending} className="mt-6 grid min-w-0 gap-3">
        <legend className="sr-only">{t('title')}</legend>
        {options.map((option) => {
          const id = `${groupId}-${option.value || 'organization'}`
          return (
            <label key={id} htmlFor={id} className="flex cursor-pointer items-start gap-3 rounded-md border p-4 has-[:checked]:border-primary-solid">
              <input
                id={id}
                type="radio"
                name={groupId}
                className="mt-1"
                checked={stored === option.value}
                onChange={() => choose(option.value)}
              />
              <span className="grid min-w-0 gap-1">
                <span className="flex flex-wrap items-center gap-2 font-medium">
                  {option.label}
                  {option.plugin && <Swatches colors={option.plugin.preview} />}
                </span>
                {option.description && <span className="text-sm text-muted-foreground">{option.description}</span>}
                {option.plugin?.has_screens && <span className="text-xs text-warning">{t('runsCode')}</span>}
              </span>
            </label>
          )
        })}
      </fieldset>
      {hasPermission(SETTINGS_MANAGE) && (
        <section className="mt-8 grid max-w-md gap-2">
          <PageHeader level="section" title={t('organizationDefault')} description={t('organizationDefaultHint')} />
          <Field id={defaultId} label={t('organizationDefault')}>
            <NativeSelect
              id={defaultId}
              value={defaultKey ?? ''}
              disabled={setDefault.isPending}
              onValueChange={(value) => setDefault.mutate(value || null)}
            >
              <option value="">{t('coreName')}</option>
              {plugins.data.map((plugin) => (
                <option key={plugin.key} value={plugin.key}>{plugin.label}</option>
              ))}
            </NativeSelect>
          </Field>
        </section>
      )}
    </Page>
  )
}
