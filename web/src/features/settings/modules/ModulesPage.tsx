import { Puzzle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useModules, useInvalidateModules } from '@/kernel/modules/api'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { ModuleCard } from './ModuleCard'
import { CatalogSourcePanel } from './CatalogSourcePanel'
import { PageHeader } from '@/components/ui/page-header'

export function ModulesPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <ModulesList />
    </RequirePermission>
  )
}

function ModulesList() {
  const { t } = useTranslation('modules')
  const modules = useModules()
  const invalidate = useInvalidateModules()
  const nameOf = (key: string) => modules.data?.find((module) => module.key === key)?.name ?? key
  const optional = modules.data?.filter((module) => !module.required) ?? []

  return (
    <section className="flex flex-col gap-4">
      <PageHeader title={t('title')} description={t('description')} />
      {modules.isPending ? (
        <div className="grid gap-3 md:grid-cols-2" aria-busy="true">
          <Skeleton className="h-56 w-full" />
          <Skeleton className="h-56 w-full" />
        </div>
      ) : modules.isError ? (
        <ErrorState onRetry={() => void modules.refetch()} />
      ) : optional.length === 0 ? (
        <EmptyState icon={Puzzle} title={t('emptyTitle')} description={t('emptyDescription')} />
      ) : (
        <ul className="grid gap-3 md:grid-cols-2" aria-label={t('listLabel')}>
          {optional.map((module) => (
            <ModuleCard key={module.key} module={module} nameOf={nameOf} onChanged={invalidate} />
          ))}
        </ul>
      )}
      <CatalogSourcePanel />
    </section>
  )
}
