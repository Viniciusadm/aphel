import { FlaskConical, History, Pencil, Plus, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { useCustomObjects } from '@/features/custom-records/api'
import { ListSkeleton } from '@/features/settings/components/ListSkeleton'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { SectionHeader } from '@/features/settings/components/SectionHeader'
import { LEAD_DISTRIBUTION_MANAGE } from '@/features/settings/lib/permissions'
import { leadDistributionApi, useDistributionMutation, useDistributionRules, type DistributionRule } from './api'
import { AssignmentHistoryDialog } from './AssignmentHistoryDialog'
import { RuleEditorDialog } from './RuleEditorDialog'
import { SimulationDialog } from './SimulationDialog'

export function LeadDistributionPage() {
  return (
    <RequirePermission permission={LEAD_DISTRIBUTION_MANAGE}>
      <LeadDistributionSection />
    </RequirePermission>
  )
}

function LeadDistributionSection() {
  const { t } = useTranslation('lead_distribution')
  const rules = useDistributionRules()
  const objects = useCustomObjects()
  const [editing, setEditing] = useState<{ rule: DistributionRule | null } | null>(null)
  const [simulating, setSimulating] = useState<DistributionRule | null>(null)
  const [deleting, setDeleting] = useState<DistributionRule | null>(null)
  const [history, setHistory] = useState<{ rule: DistributionRule | null } | null>(null)
  const remove = useDistributionMutation((rule: DistributionRule) => leadDistributionApi.remove(rule.id))

  const recordTypeLabel = (recordType: string) => {
    if (recordType === 'deal' || recordType === 'contact' || recordType === 'company') {
      return t(`recordTypes.${recordType}`)
    }
    return objects.data?.find((object) => object.record_type === recordType)?.label_plural ?? recordType
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
    <section className="grid gap-4">
      <SectionHeader
        title={t('title')}
        description={t('description')}
        action={
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" onClick={() => setHistory({ rule: null })}>
              <History />
              {t('history.open')}
            </Button>
            <Button onClick={() => setEditing({ rule: null })}>
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
        <div className="relative overflow-x-auto rounded-lg border">
          <table className="w-full text-sm">
            <thead className="bg-muted/50 text-left text-muted-foreground">
              <tr>
                <th scope="col" className="px-4 py-2 font-medium">
                  {t('columns.name')}
                </th>
                <th scope="col" className="hidden px-4 py-2 font-medium sm:table-cell">
                  {t('columns.recordType')}
                </th>
                <th scope="col" className="hidden px-4 py-2 font-medium md:table-cell">
                  {t('columns.strategy')}
                </th>
                <th scope="col" className="hidden px-4 py-2 font-medium lg:table-cell">
                  {t('columns.members')}
                </th>
                <th scope="col" className="hidden px-4 py-2 font-medium lg:table-cell">
                  {t('columns.assignments')}
                </th>
                <th scope="col" className="px-4 py-2 font-medium">
                  {t('columns.status')}
                </th>
                <th scope="col" className="w-40 px-4 py-2">
                  <span className="sr-only">{t('columns.actions')}</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {rules.data.map((rule) => {
                const away = rule.members.filter((member) => member.away_now || !member.active).length
                const last = memberName(rule, rule.last_user_id)
                return (
                  <tr key={rule.id} className="border-t align-top">
                    <td className="px-4 py-3">
                      <div className="font-medium">{rule.name}</div>
                      {rule.routing && (
                        <div className="text-xs text-muted-foreground">
                          {t('routed', { field: rule.routing.field })}
                        </div>
                      )}
                    </td>
                    <td className="hidden px-4 py-3 text-muted-foreground sm:table-cell">
                      {recordTypeLabel(rule.record_type)}
                    </td>
                    <td className="hidden px-4 py-3 md:table-cell">{t(`strategies.${rule.strategy}`)}</td>
                    <td className="hidden px-4 py-3 lg:table-cell">
                      <div>{t('membersSummary', { count: rule.members.length })}</div>
                      {away > 0 && (
                        <div className="text-xs text-muted-foreground">{t('awaySummary', { count: away })}</div>
                      )}
                    </td>
                    <td className="hidden px-4 py-3 lg:table-cell">
                      <div className="tabular-nums">{rule.assignments}</div>
                      {last && <div className="text-xs text-muted-foreground">{t('lastAssigned', { name: last })}</div>}
                    </td>
                    <td className="px-4 py-3">
                      <Badge variant={rule.is_active ? 'success' : 'secondary'}>
                        {t(rule.is_active ? 'status.active' : 'status.inactive')}
                      </Badge>
                    </td>
                    <td className="px-4 py-3">
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
                          onClick={() => setEditing({ rule })}
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
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
      {editing && <RuleEditorDialog rule={editing.rule} onClose={() => setEditing(null)} />}
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
    </section>
  )
}
