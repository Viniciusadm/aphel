import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useMigrations } from './api'
import { SectionCard } from './SectionCard'
import type { InstalledModule, Migrations } from './types'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

function MigrationsBody({ migrations }: { migrations: Migrations }) {
  const { t } = useTranslation('system')
  const pendingEntries = [
    ...migrations.core.pending.map((entry) => ({ scope: 'core', ...entry })),
    ...migrations.modules.flatMap((module) =>
      module.pending.map((entry) => ({ scope: module.key, ...entry })),
    ),
  ]

  return (
    <>
      <p className="text-sm">
        {t('migrations.summary', {
          applied: migrations.core.applied,
          known: migrations.core.known,
          latest: migrations.core.latest_applied ?? t('migrations.none'),
        })}{' '}
        {migrations.pending_total === 0
          ? t('migrations.upToDate')
          : t('migrations.pending', { count: migrations.pending_total })}
      </p>
      {migrations.problems.length > 0 && (
        <div role="alert" className="grid gap-1 rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm">
          <span className="font-medium">{t('migrations.problems')}</span>
          <ul className="list-disc pl-5">
            {migrations.problems.map((problem) => (
              <li key={problem} className="break-words">
                {problem}
              </li>
            ))}
          </ul>
        </div>
      )}
      {pendingEntries.length > 0 && (
        <div className="grid gap-1 text-sm">
          <span className="font-medium">{t('migrations.pendingList')}</span>
          <ul className="grid gap-0.5 font-mono text-xs">
            {pendingEntries.map((entry) => (
              <li key={`${entry.scope}:${entry.version}`} className="break-words">
                {entry.scope} {entry.version} {entry.description}
              </li>
            ))}
          </ul>
        </div>
      )}
    </>
  )
}

export function MigrationsSection() {
  const { t } = useTranslation('system')
  const migrations = useMigrations()

  let body
  if (migrations.isPending) {
    body = <Skeleton className="h-20 w-full" />
  } else if (migrations.isError) {
    body = <ErrorState onRetry={() => void migrations.refetch()} />
  } else {
    body = <MigrationsBody migrations={migrations.data} />
  }
  return <SectionCard title={t('migrations.title')}>{body}</SectionCard>
}

export function ModulesSection({ modules }: { modules: InstalledModule[] }) {
  const { t } = useTranslation('system')
  const migrations = useMigrations()
  const counts = new Map(
    (migrations.data?.modules ?? []).map((module) => [
      module.key,
      { applied: module.applied, pending: module.pending.length },
    ]),
  )

  return (
    <SectionCard title={t('modules.title')}>
      {modules.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t('modules.empty')}</p>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">
                {t('modules.columns.module')}
              </TableHead>
              <TableHead scope="col" className="hidden sm:table-cell">
                {t('modules.columns.version')}
              </TableHead>
              <TableHead scope="col">
                {t('modules.columns.compatibility')}
              </TableHead>
              <TableHead scope="col" className="text-right">
                {t('modules.columns.enabled')}
              </TableHead>
              <TableHead scope="col" className="hidden md:table-cell">
                {t('modules.columns.migrations')}
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {modules.map((module) => {
              const compatible = module.compatibility === 'compatible'
              const count = counts.get(module.key)
              return (
                <TableRow key={module.key} className="align-top">
                  <TableCell>
                    <div className="grid gap-0.5">
                      <span>{module.name}</span>
                      <span className="font-mono text-xs text-muted-foreground">{module.key}</span>
                    </div>
                  </TableCell>
                  <TableCell className="hidden font-mono text-xs sm:table-cell">{module.version}</TableCell>
                  <TableCell>
                    <Badge
                      variant={compatible ? 'success' : 'destructive'}
                      title={module.compatibility_detail ?? module.compatibility}
                    >
                      {t(compatible ? 'modules.compatible' : 'modules.incompatible')}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-right">
                    {t(module.enabled ? 'modules.enabled' : 'modules.disabled')}
                  </TableCell>
                  <TableCell className="hidden text-xs text-muted-foreground md:table-cell">
                    {count ? t('modules.migrationsCount', count) : '—'}
                  </TableCell>
                </TableRow>
              )
            })}
          </TableBody>
        </Table>
      )}
    </SectionCard>
  )
}
