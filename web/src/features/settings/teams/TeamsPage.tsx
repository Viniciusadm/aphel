import { Pencil, Plus, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { toast } from '@/components/ui/toast-store'
import { settingsApi, useAllMembers, useSettingsMutation, useTeams, type Team } from '../api'
import { ListSkeleton } from '../components/ListSkeleton'
import { RequirePermission } from '../components/RequirePermission'
import { errorMessageKey } from '../lib/errors'
import { SETTINGS_MANAGE } from '../lib/permissions'
import { TeamEditorDialog } from './TeamEditorDialog'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Page } from '@/components/ui/page'

export function TeamsPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <TeamsSection />
    </RequirePermission>
  )
}

function TeamsSection() {
  const { t } = useTranslation('settings')
  const teams = useTeams()
  const members = useAllMembers()
  const [editing, setEditing] = useState<{ team: Team | null } | null>(null)
  const [deleting, setDeleting] = useState<Team | null>(null)
  const remove = useSettingsMutation((team: Team) => settingsApi.deleteTeam(team.id))

  const confirmDelete = async (team: Team) => {
    try {
      await remove.mutateAsync(team)
      toast({ title: t('teams.toast.deleted'), variant: 'success' })
    } catch (error) {
      toast({ title: t(errorMessageKey(error)), variant: 'destructive' })
    } finally {
      setDeleting(null)
    }
  }

  const teamName = (id: string | null | undefined) =>
    (id && teams.data?.find((team) => team.id === id)?.name) || t('teams.noParent')

  return (
    <Page>
      <PageHeader
        title={t('teams.title')}
        description={t('teams.description')}
        actions={
          <Button onClick={() => setEditing({ team: null })} disabled={!teams.data || !members.data}>
            <Plus />
            {t('teams.create')}
          </Button>
        }
      />
      {teams.isPending ? (
        <ListSkeleton />
      ) : teams.isError ? (
        <ErrorState onRetry={() => void teams.refetch()} />
      ) : teams.data.length === 0 ? (
        <EmptyState title={t('teams.empty')} />
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">
                {t('teams.columns.name')}
              </TableHead>
              <TableHead scope="col" className="hidden sm:table-cell">
                {t('teams.columns.parent')}
              </TableHead>
              <TableHead scope="col" className="hidden md:table-cell">
                {t('teams.columns.managers')}
              </TableHead>
              <TableHead scope="col">
                {t('teams.columns.members')}
              </TableHead>
              <TableHead scope="col" className="w-24">
                <span className="sr-only">{t('teams.columns.actions')}</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {teams.data.map((team) => (
              <TableRow key={team.id}>
                <TableCell className="font-medium">{team.name}</TableCell>
                <TableCell className="hidden text-muted-foreground sm:table-cell">
                  {teamName(team.parent_team_id)}
                </TableCell>
                <TableCell className="hidden text-muted-foreground md:table-cell">
                  {team.members
                    .filter((member) => member.is_manager)
                    .map((member) => member.display_name)
                    .join(', ') || t('teams.noParent')}
                </TableCell>
                <TableCell>{t('teams.membersCount', { count: team.members.length })}</TableCell>
                <TableCell>
                  <div className="flex justify-end gap-1">
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label={`${t('actions.edit')}: ${team.name}`}
                      onClick={() => setEditing({ team })}
                      disabled={!members.data}
                    >
                      <Pencil />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label={`${t('actions.delete')}: ${team.name}`}
                      onClick={() => setDeleting(team)}
                    >
                      <Trash2 />
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
      {editing && teams.data && members.data && (
        <TeamEditorDialog
          team={editing.team}
          teams={teams.data}
          members={members.data}
          onClose={() => setEditing(null)}
        />
      )}
      {deleting && (
        <ConfirmDialog
          open
          title={t('teams.confirmDelete.title', { name: deleting.name })}
          description={t('teams.confirmDelete.description')}
          confirmLabel={t('actions.delete')}
          pending={remove.isPending}
          variant="destructive"
          onConfirm={() => confirmDelete(deleting)}
          onOpenChange={(open) => !open && setDeleting(null)}
        />
      )}
    </Page>
  )
}
