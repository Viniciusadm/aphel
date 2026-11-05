import { useAssignmentTargets } from './api'
import i18n from '@/i18n'
import { FlaskConical, History, Pencil, Plus, Trash2 } from 'lucide-react'
import { useNavigate } from 'react-router'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { ASSIGNMENT_MANAGE, ListSkeleton, RequirePermission } from '@/sdk'
import { leadDistributionApi, useDistributionMutation, useDistributionRules, type DistributionRule } from './api'
import { AssignmentHistoryDialog } from './AssignmentHistoryDialog'
import { SimulationDialog } from './SimulationDialog'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Page } from '@/components/ui/page'

export function LeadDistributionPage() {
  return (
    <RequirePermission permission={ASSIGNMENT_MANAGE}>
      <LeadDistributionSection />
    </RequirePermission>
  )
}

function LeadDistributionSection() {
  const { t } = useTranslation('assignment')
  const rules = useDistributionRules()
  const targets = useAssignmentTargets()
  const navigate = useNavigate()
  const [simulating, setSimulating] = useState<DistributionRule | null>(null)
  const [deleting, setDeleting] = useState<DistributionRule | null>(null)
  const [history, setHistory] = useState<{ rule: DistributionRule | null } | null>(null)
  const remove = useDistributionMutation((rule: DistributionRule) => leadDistributionApi.remove(rule.id))

  const recordTypeLabel = (recordType:string) => {
    const target=targets.data?.find((target)=>target.record_type===recordType)
    return target?.labels[i18n.resolvedLanguage ?? i18n.language] ?? target?.label ?? recordType
  }

  const confirmDelete = async (rule: DistributionRule) => {
    try {
      await remove.mutateAsync(rule)
      notifySuccess(t('toast.deleted'))
    } catch (error) {
      notifyError(error)
    } finally {
      setDeleting(null)
    }
  }

  const memberName = (rule: DistributionRule, userId: string | null | undefined) =>
    rule.members.find((member) => member.user_id === userId)?.name ?? null

  return (
    <Page>
      <PageHeader
        title={t('title')}
        description={t('description')}
        actions={
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" onClick={() => setHistory({ rule: null })}>
              <History />
              {t('history.open')}
            </Button>
            <Button onClick={() => void navigate('/settings/assignment/new')}>
              <Plus />
              {t('create')}
            </Button>
          </div>
        }
      />
      {rules.isPending ? (
        <ListSkeleton />
      ) : rules.isError ? (
        <ErrorState onRetry={() => void rules.refetch()} />
      ) : rules.data.length === 0 ? (
        <EmptyState title={t('empty')} description={t('emptyDescription')} />
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">
                {t('columns.name')}
              </TableHead>
              <TableHead scope="col" className="hidden sm:table-cell">
                {t('columns.recordType')}
              </TableHead>
              <TableHead scope="col" className="hidden md:table-cell">
                {t('columns.strategy')}
              </TableHead>
              <TableHead scope="col" className="hidden lg:table-cell">
                {t('columns.members')}
              </TableHead>
              <TableHead scope="col" className="hidden lg:table-cell">
                {t('columns.assignments')}
              </TableHead>
              <TableHead scope="col">
                {t('columns.status')}
              </TableHead>
              <TableHead scope="col" className="w-40">
                <span className="sr-only">{t('columns.actions')}</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rules.data.map((rule) => {
              const away = rule.members.filter((member) => member.away_now || !member.active).length
              const last = memberName(rule, rule.last_user_id)
              return (
                <TableRow key={rule.id} className="align-top">
                  <TableCell>
                    <div className="font-medium">{rule.name}</div>
                    {rule.routing && (
                      <div className="text-xs text-muted-foreground">
                        {t('routed', { field: rule.routing.field })}
                      </div>
                    )}
                  </TableCell>
                  <TableCell className="hidden text-muted-foreground sm:table-cell">
                    {recordTypeLabel(rule.record_type)}
                  </TableCell>
                  <TableCell className="hidden md:table-cell">{t(`strategies.${rule.strategy}`)}</TableCell>
                  <TableCell className="hidden lg:table-cell">
                    <div>{t('membersSummary', { count: rule.members.length })}</div>
                    {away > 0 && (
                      <div className="text-xs text-muted-foreground">{t('awaySummary', { count: away })}</div>
                    )}
                  </TableCell>
                  <TableCell className="hidden lg:table-cell">
                    <div className="tabular-nums">{rule.assignments}</div>
                    {last && <div className="text-xs text-muted-foreground">{t('lastAssigned', { name: last })}</div>}
                  </TableCell>
                  <TableCell>
                    <Badge variant={rule.is_active ? 'success' : 'secondary'}>
                      {t(rule.is_active ? 'status.active' : 'status.inactive')}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex justify-end gap-1">
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={`${t('actions.simulate')}: ${rule.name}`}
                        onClick={() => setSimulating(rule)}
                      >
                        <FlaskConical />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={`${t('actions.history')}: ${rule.name}`}
                        onClick={() => setHistory({ rule })}
                      >
                        <History />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={`${t('actions.edit')}: ${rule.name}`}
                        onClick={() => void navigate(`/settings/assignment/${encodeURIComponent(rule.id)}/edit`)}
                      >
                        <Pencil />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={`${t('actions.delete')}: ${rule.name}`}
                        onClick={() => setDeleting(rule)}
                      >
                        <Trash2 />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              )
            })}
          </TableBody>
        </Table>
      )}
      {history && <AssignmentHistoryDialog rule={history.rule} onClose={() => setHistory(null)} />}
      {simulating && <SimulationDialog rule={simulating} onClose={() => setSimulating(null)} />}
      {deleting && (
        <ConfirmDialog
          open
          variant="destructive"
          title={t('confirmDelete.title', { name: deleting.name })}
          description={t('confirmDelete.description')}
          confirmLabel={t('actions.delete')}
          pending={remove.isPending}
          onConfirm={() => confirmDelete(deleting)}
          onOpenChange={(open) => !open && setDeleting(null)}
        />
      )}
    </Page>
  )
}
