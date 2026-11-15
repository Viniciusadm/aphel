import { Puzzle } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { NativeSelect } from '@/components/ui/native-select'
import { SearchInput } from '@/components/ui/search-input'
import { Skeleton } from '@/components/ui/skeleton'
import { useModules, useInvalidateModules } from '@/kernel/modules/api'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { useAllModuleTranslations } from '@/kernel/modules/i18n'
import { moduleHasScreens } from '@/kernel/modules/surface'
import { ModuleCard } from './ModuleCard'
import { useModuleName } from './text'
import { Slot } from '@/kernel/modules/Slot'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'
import { pluginManagementAllowed } from '@/kernel/modules/wasm_runtime'

export function ModulesPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      {pluginManagementAllowed() ? <ModulesList /> : <ModulesDisabled />}
    </RequirePermission>
  )
}

function ModulesDisabled() {
  const { t } = useTranslation('modules')
  return (
    <Page>
      <EmptyState icon={Puzzle} title={t('managementDisabledTitle')} description={t('managementDisabledDescription')} />
    </Page>
  )
}

type StateFilter = 'all' | 'enabled' | 'disabled'

function ModulesList() {
  const { t } = useTranslation('modules')
  const modules = useModules()
  const invalidate = useInvalidateModules()
  const moduleName = useModuleName()
  const translationsLoaded = useAllModuleTranslations()
  const [query, setQuery] = useState('')
  const [state, setState] = useState<StateFilter>('all')
  const all = useMemo(() => modules.data ?? [], [modules.data])
  const nameOf = (key: string) => {
    const module = all.find((item) => item.key === key)
    return module ? moduleName(module) : key
  }

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase()
    return all
      .filter((module) => !module.required)
      .filter((module) => state === 'all' || (state === 'enabled') === module.enabled)
      .map((module) => ({ module, name: moduleName(module) }))
      .filter(({ module, name }) => needle === '' || name.toLowerCase().includes(needle) || module.key.includes(needle))
      .sort((left, right) => Number(right.module.enabled) - Number(left.module.enabled) || left.name.localeCompare(right.name))
      .map(({ module }) => module)
  }, [all, moduleName, query, state])

  const withScreens = visible.filter((module) => moduleHasScreens(module.key))
  const support = visible.filter((module) => !moduleHasScreens(module.key))
  const sections = [
    { id: 'screens', items: withScreens },
    { id: 'support', items: support },
  ] as const

  return (
    <Page>
      <PageHeader title={t('title')} description={t('description')} />
      {modules.isPending || !translationsLoaded ? (
        <div className="grid gap-3 md:grid-cols-2" aria-busy="true">
          <Skeleton className="h-56 w-full" />
          <Skeleton className="h-56 w-full" />
        </div>
      ) : modules.isError ? (
        <ErrorState onRetry={() => void modules.refetch()} />
      ) : all.every((module) => module.required) ? (
        <EmptyState icon={Puzzle} title={t('emptyTitle')} description={t('emptyDescription')} />
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-3">
            <SearchInput
              value={query}
              onValueChange={setQuery}
              placeholder={t('filters.searchPlaceholder')}
              aria-label={t('filters.search')}
              className="w-full sm:w-72"
            />
            <NativeSelect
              aria-label={t('filters.state')}
              value={state}
              onChange={(event) => setState(event.target.value as StateFilter)}
              className="w-full sm:w-48"
            >
              <option value="all">{t('filters.all')}</option>
              <option value="enabled">{t('filters.enabled')}</option>
              <option value="disabled">{t('filters.disabled')}</option>
            </NativeSelect>
          </div>
          {visible.length === 0 ? (
            <EmptyState icon={Puzzle} title={t('filters.noResults')} description={t('filters.noResultsDescription')} />
          ) : (
            sections
              .filter((section) => section.items.length > 0)
              .map((section) => (
                <section key={section.id} className="flex flex-col gap-4" aria-labelledby={`modules-${section.id}`}>
                  <PageHeader
                    level="section"
                    title={<span id={`modules-${section.id}`}>{t(`sections.${section.id}.title`)}</span>}
                    description={t(`sections.${section.id}.description`)}
                  />
                  <ul className="divide-y rounded-lg border bg-card" aria-label={t(`sections.${section.id}.title`)}>
                    {section.items.map((module) => (
                      <ModuleCard key={module.key} module={module} modules={all} nameOf={nameOf} onChanged={invalidate} />
                    ))}
                  </ul>
                </section>
              ))
          )}
        </>
      )}
      <Slot name="modules.settings" props={{}} />
    </Page>
  )
}
