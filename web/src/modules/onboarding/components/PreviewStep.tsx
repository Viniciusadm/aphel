import { TriangleAlert } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { usePackageLabels } from '@/features/settings/config-packages/lib/labels'
import type { PackagePlan, PlanItem } from '../api'
import { EmptyState } from '@/components/ui/empty-state'

const SUMMARY_KEYS = ['create', 'update', 'skip_conflict', 'unchanged'] as const

interface PreviewStepProps {
  plan: PackagePlan | undefined
  isPending: boolean
  isError: boolean
  onRetry: () => void
}

function groupBySection(plan: PackagePlan): Array<[string, PlanItem[]]> {
  const changing = plan.items.filter((item) => item.action === 'create' || item.action === 'update')
  const order = [...plan.sections, ...changing.map((item) => item.section)]
  const sections = [...new Set(order)]
  return sections
    .map((section): [string, PlanItem[]] => [section, changing.filter((item) => item.section === section)])
    .filter(([, items]) => items.length > 0)
}

export function PreviewStep({ plan, isPending, isError, onRetry }: PreviewStepProps) {
  const { t } = useTranslation('onboarding')
  const labels = usePackageLabels()

  if (isPending) {
    return (
      <div className="grid gap-3" aria-busy="true">
        <span className="sr-only" role="status">
          {t('preview.loading')}
        </span>
        <Skeleton className="h-16 w-full" />
        <Skeleton className="h-32 w-full" />
      </div>
    )
  }
  if (isError || !plan) {
    return <ErrorState description={t('errors.plan')} onRetry={onRetry} />
  }
  if (!plan.valid) {
    return <ErrorState description={t('errors.invalidTemplate')} onRetry={onRetry} />
  }

  const groups = groupBySection(plan)
  const conflicts = plan.items.filter((item) => item.action === 'skip_conflict')

  return (
    <div className="grid gap-5">
      <dl className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        {SUMMARY_KEYS.map((key) => (
          <div key={key} className="rounded-lg border bg-card p-3">
            <dt className="text-xs text-muted-foreground">{t(`preview.summary.${key}`)}</dt>
            <dd className="text-xl font-semibold tabular-nums">{plan.summary[key]}</dd>
          </div>
        ))}
      </dl>

      {groups.length === 0 ? (
        <EmptyState title={t('preview.nothing')} />
      ) : (
        <div className="grid gap-4">
          {groups.map(([section, items]) => (
            <section key={section} className="grid gap-2" aria-labelledby={`onboarding-section-${section}`}>
              <h3 id={`onboarding-section-${section}`} className="text-sm font-semibold">
                {labels.section(section)}
              </h3>
              <ul className="grid gap-1.5">
                {items.map((item) => (
                  <li
                    key={item.id}
                    className="flex flex-wrap items-center justify-between gap-2 rounded-md border bg-card px-3 py-2 text-sm"
                  >
                    <span className="min-w-0 break-words">
                      <span className="text-muted-foreground">
                        {labels.kind(item.kind)}
                        {': '}
                      </span>
                      {item.label}
                    </span>
                    <Badge variant={item.action === 'create' ? 'success' : 'secondary'}>
                      {t(`preview.actions.${item.action as 'create' | 'update'}`)}
                    </Badge>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
      )}

      {plan.skipped_sections.length > 0 && (
        <EmptyState title={t('preview.skipped', { sections: plan.skipped_sections.map((section) => labels.section(section)).join(', ') })} />
      )}

      {plan.skipped_fields.length > 0 && (
        <EmptyState title={t('preview.skippedFields')} />
      )}

      {conflicts.length > 0 && (
        <section
          className="grid gap-2 rounded-lg border border-warning/40 bg-warning/10 p-4"
          aria-labelledby="onboarding-conflicts"
        >
          <h3 id="onboarding-conflicts" className="flex items-center gap-2 text-sm font-semibold">
            <TriangleAlert className="size-4" aria-hidden />
            {t('preview.conflicts.title', { count: conflicts.length })}
          </h3>
          <p className="text-sm text-muted-foreground">{t('preview.conflicts.description')}</p>
          <ul className="grid gap-1 text-sm">
            {conflicts.map((item) => (
              <li key={item.id} className="break-words">
                {labels.kind(item.kind)}
                {': '}
                {item.label}
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  )
}
