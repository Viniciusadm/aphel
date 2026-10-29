import { UserPlus } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { toast } from '@/components/ui/toast-store'
import {
  MEMBERS_PER_PAGE,
  settingsApi,
  useMembersPage,
  useRoles,
  useSettingsMutation,
  type InvitedMember,
  type Member,
} from '../api'
import { ListSkeleton } from '../components/ListSkeleton'
import { Pagination } from '../components/Pagination'
import { RequirePermission } from '../components/RequirePermission'
import { SectionHeader } from '../components/SectionHeader'
import { errorMessageKey } from '../lib/errors'
import { USERS_MANAGE } from '../lib/permissions'
import { ChangeRoleDialog } from './ChangeRoleDialog'
import { InviteMemberDialog } from './InviteMemberDialog'
import { MemberLinkDialog, type MemberLink } from './MemberLinkDialog'
import { MemberActions, type MemberAction } from './MemberActions'
import { MemberStatusBadge } from './MemberStatusBadge'

type ConfirmAction = Extract<MemberAction, 'suspend' | 'cancelInvitation'>

export function UsersPage() {
  return (
    <RequirePermission permission={USERS_MANAGE}>
      <MembersSection />
    </RequirePermission>
  )
}

function MembersSection() {
  const { t } = useTranslation('settings')
  const { me } = useSession()
  const [page, setPage] = useState(1)
  const [inviting, setInviting] = useState(false)
  const [roleTarget, setRoleTarget] = useState<Member | null>(null)
  const [confirming, setConfirming] = useState<{ action: ConfirmAction; member: Member } | null>(null)
  const [memberLink, setMemberLink] = useState<MemberLink | null>(null)
  const members = useMembersPage(page)
  const roles = useRoles()

  const invitationLink = (member: InvitedMember) => {
    if (!member.invitation_link) return
    setMemberLink({
      title: t('users.linkDialog.invitationTitle', { email: member.email }),
      description: t('users.linkDialog.invitationDescription'),
      link: member.invitation_link,
    })
  }

  const run = useSettingsMutation(async ({ action, member }: { action: MemberAction; member: Member }) => {
    switch (action) {
      case 'resend': {
        const resent = await settingsApi.resendInvitation(member.id)
        if (resent.invitation_link) {
          invitationLink(resent)
          return null
        }
        return t('users.toast.resent', { email: member.email })
      }
      case 'resetPassword': {
        const reset = await settingsApi.resetMemberPassword(member.id)
        if (reset.link) {
          setMemberLink({
            title: t('users.linkDialog.resetTitle', { name: member.display_name }),
            description: t('users.linkDialog.resetDescription'),
            link: reset.link,
          })
          return null
        }
        return reset.emailed ? t('users.toast.resetEmailed', { email: member.email }) : t('users.toast.resetUnavailable')
      }
      case 'cancelInvitation':
        await settingsApi.cancelInvitation(member.id)
        return t('users.toast.cancelled')
      case 'suspend':
        await settingsApi.suspendMember(member.id)
        return t('users.toast.suspended')
      default:
        await settingsApi.reactivateMember(member.id)
        return t('users.toast.reactivated')
    }
  })

  const execute = async (action: MemberAction, member: Member) => {
    try {
      const message = await run.mutateAsync({ action, member })
      if (message) toast({ title: message, variant: 'success' })
    } catch (error) {
      toast({ title: t(errorMessageKey(error)), variant: 'destructive' })
    } finally {
      setConfirming(null)
    }
  }

  const onAction = (action: MemberAction, member: Member) => {
    if (action === 'changeRole') setRoleTarget(member)
    else if (action === 'suspend' || action === 'cancelInvitation') setConfirming({ action, member })
    else void execute(action, member)
  }

  const confirmCopy = confirming && {
    suspend: {
      title: t('users.confirm.suspendTitle', { name: confirming.member.display_name }),
      description: t('users.confirm.suspendDescription'),
      label: t('users.menu.suspend'),
    },
    cancelInvitation: {
      title: t('users.confirm.cancelTitle', { email: confirming.member.email }),
      description: t('users.confirm.cancelDescription'),
      label: t('users.menu.cancelInvitation'),
    },
  }[confirming.action]

  return (
    <section className="grid gap-4">
      <SectionHeader
        title={t('users.title')}
        description={t('users.description')}
        action={
          <Button onClick={() => setInviting(true)} disabled={!roles.data}>
            <UserPlus />
            {t('users.invite')}
          </Button>
        }
      />
      {members.isPending ? (
        <ListSkeleton />
      ) : members.isError ? (
        <ErrorState onRetry={() => void members.refetch()} />
      ) : members.data.data.length === 0 ? (
        <EmptyState title={t('users.empty')} />
      ) : (
        <>
          <div className="relative overflow-x-auto rounded-lg border">
            <table className="w-full text-sm">
              <thead className="bg-muted/50 text-left text-muted-foreground">
                <tr>
                  <th scope="col" className="px-4 py-2 font-medium">
                    {t('users.columns.name')}
                  </th>
                  <th scope="col" className="px-4 py-2 font-medium">
                    {t('users.columns.role')}
                  </th>
                  <th scope="col" className="px-4 py-2 font-medium">
                    {t('users.columns.status')}
                  </th>
                  <th scope="col" className="hidden px-4 py-2 font-medium md:table-cell">
                    {t('users.columns.teams')}
                  </th>
                  <th scope="col" className="w-12 px-4 py-2">
                    <span className="sr-only">{t('users.columns.actions')}</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {members.data.data.map((member) => (
                  <tr key={member.id} className="border-t">
                    <td className="px-4 py-3">
                      <div className="font-medium">
                        {member.display_name}
                        {member.user_id === me.user.id && (
                          <span className="ml-1 text-muted-foreground">({t('users.you')})</span>
                        )}
                      </div>
                      <div className="text-muted-foreground">{member.email}</div>
                    </td>
                    <td className="px-4 py-3">{member.role.name}</td>
                    <td className="px-4 py-3">
                      <MemberStatusBadge status={member.status} />
                    </td>
                    <td className="hidden px-4 py-3 text-muted-foreground md:table-cell">
                      {member.teams.length > 0 ? member.teams.map((team) => team.name).join(', ') : t('users.noTeams')}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <MemberActions member={member} onAction={onAction} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <Pagination page={page} perPage={MEMBERS_PER_PAGE} total={members.data.meta.total} onPageChange={setPage} />
        </>
      )}
      <InviteMemberDialog
        open={inviting}
        roles={roles.data ?? []}
        onOpenChange={setInviting}
        onInvited={invitationLink}
      />
      {memberLink && <MemberLinkDialog value={memberLink} onClose={() => setMemberLink(null)} />}
      {roleTarget && (
        <ChangeRoleDialog member={roleTarget} roles={roles.data ?? []} onClose={() => setRoleTarget(null)} />
      )}
      {confirming && confirmCopy && (
        <ConfirmDialog
          open
          title={confirmCopy.title}
          description={confirmCopy.description}
          confirmLabel={confirmCopy.label}
          pending={run.isPending}
          variant="destructive"
          onConfirm={() => execute(confirming.action, confirming.member)}
          onOpenChange={(open) => !open && setConfirming(null)}
        />
      )}
    </section>
  )
}
