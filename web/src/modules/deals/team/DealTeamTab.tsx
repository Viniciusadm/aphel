import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { Plus, Users } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { UserFieldInput } from '@/components/fields'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Avatar } from '@/components/ui/avatar'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { useOrganizationUsers } from '@/features/records/api'
import { useDealMemberRoles, useDealTeam, useDealTeamMutations } from './api'
import { dealTeamErrorMessage, formatPercent, parsePercent, toPercentInput } from './errors'
import { MEMBER_GRID_CLASS, MemberRow } from './MemberRow'
import { RoleSelect } from './RoleSelect'
import type { DealMember, DealMemberRole, DealTeam, UpdateDealMemberRequest } from './types'
import { Field } from '@/components/ui/field'

export interface DealTeamTabProps {
  dealId: string
  readOnly?: boolean
}

export function DealTeamTab({ dealId, readOnly = false }: DealTeamTabProps) {
  const { t } = useTranslation('deals', { keyPrefix: 'team' })
  const { me, permissionScope } = useSession()
  const team = useDealTeam(dealId)
  const roles = useDealMemberRoles(false)
  const mutations = useDealTeamMutations(dealId)
  const [adding, setAdding] = useState(false)
  const [removing, setRemoving] = useState<DealMember | null>(null)

  if (team.isPending) {
    return (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-16 w-full" />
        <Skeleton className="h-12 w-full" />
      </div>
    )
  }
  if (team.isError) {
    return <ErrorState title={t('loadError')} onRetry={() => void team.refetch()} />
  }

  const data = team.data
  const locked = readOnly || (permissionScope('deals.update') === 'own' && data.owner?.id !== me.user.id)
  const busy = mutations.add.isPending || mutations.update.isPending || mutations.remove.isPending
  const fail = (error: unknown) => toast({ title: dealTeamErrorMessage(error, t), variant: 'destructive' })

  const save = async (member: DealMember, body: UpdateDealMemberRequest) => {
    try {
      await mutations.update.mutateAsync({ member, body })
      notifySuccess(t('saved'))
    } catch (error) {
      fail(error)
      throw error
    }
  }

  const remove = async () => {
    if (!removing) return
    try {
      await mutations.remove.mutateAsync(removing)
      notifySuccess(t('removed', { name: removing.user.display_name }))
      setRemoving(null)
    } catch (error) {
      fail(error)
    }
  }

  return (
    <div className="@container grid min-w-0 gap-4">
      <p className="text-sm text-muted-foreground">{t('description')}</p>
      <CreditSummary team={data} />

      {!locked && !adding && (
        <div>
          <Button size="sm" variant="outline" disabled={busy} onClick={() => setAdding(true)}>
            <Plus aria-hidden="true" />
            {t('add.open')}
          </Button>
        </div>
      )}

      {!locked && adding && (
        <AddMemberForm
          team={data}
          roles={roles.data ?? []}
          busy={busy}
          onCancel={() => setAdding(false)}
          onSubmit={async (body, name) => {
            try {
              await mutations.add.mutateAsync(body)
              notifySuccess(t('added', { name }))
              setAdding(false)
            } catch (error) {
              fail(error)
            }
          }}
        />
      )}

      {data.members.length === 0 ? (
        <EmptyState icon={Users} title={t('empty')} description={t('emptyDescription')} />
      ) : (
        <div className="min-w-0">
          <div
            aria-hidden="true"
            className={`hidden border-b px-2 pb-2 text-xs font-medium text-muted-foreground @2xl:grid ${MEMBER_GRID_CLASS}`}
          >
            <span>{t('columns.member')}</span>
            <span>{t('columns.role')}</span>
            <span className="text-right">{t('columns.credit')}</span>
            <span />
          </div>
          <ul aria-label={t('title')} className="grid gap-2 @2xl:gap-0">
            {data.members.map((member) => (
              <MemberRow
                key={`${member.id}-${member.updated_at}`}
                member={member}
                isOwner={member.user.id === data.owner?.id}
                roles={roles.data ?? []}
                readOnly={locked}
                busy={busy}
                onSave={save}
                onRemove={setRemoving}
              />
            ))}
          </ul>
        </div>
      )}

      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => !open && setRemoving(null)}
        title={t('removeTitle')}
        description={t('removeDescription', { name: removing?.user.display_name ?? '' })}
        confirmLabel={t('removeConfirm')}
        variant="destructive"
        pending={mutations.remove.isPending}
        notifyOnError={false}
        onConfirm={remove}
      />
    </div>
  )
}

function CreditSummary({ team }: { team: DealTeam }) {
  const { t, i18n } = useTranslation('deals', { keyPrefix: 'team' })
  const percent = (value: string | number) => formatPercent(value, i18n.language)
  const allocated = Math.min(parsePercent(team.allocated_percent), 100)
  const ownerShare = team.owner ? parsePercent(team.owner.credit_percent) : 0
  const ownerOnly = Math.max(100 - allocated, 0)

  return (
    <div className="grid gap-3 rounded-md border p-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          {team.owner ? (
            <>
              <Avatar name={team.owner.display_name} />
              <div className="grid min-w-0">
                <span className="truncate text-sm font-medium">{team.owner.display_name}</span>
                <span className="text-xs text-muted-foreground">{t('summary.owner')}</span>
              </div>
            </>
          ) : (
            <span className="text-sm text-muted-foreground">{t('summary.noOwner')}</span>
          )}
        </div>
        {team.owner && (
          <span className="text-sm font-semibold tabular-nums">
            {t('summary.ownerShare', { percent: percent(ownerShare) })}
          </span>
        )}
      </div>
      <div
        role="img"
        aria-label={t('summary.barLabel', { owner: percent(ownerOnly), members: percent(allocated) })}
        className="flex h-2 w-full overflow-hidden rounded-full bg-muted"
      >
        <span className="h-full bg-primary-solid" style={{ width: `${ownerOnly}%` }} />
        <span className="h-full bg-success-solid" style={{ width: `${allocated}%` }} />
      </div>
      <dl className="flex flex-wrap gap-x-6 gap-y-1 text-sm">
        <div className="flex gap-2">
          <dt className="text-muted-foreground">{t('summary.allocated')}</dt>
          <dd className="tabular-nums">{percent(team.allocated_percent)}</dd>
        </div>
        <div className="flex gap-2">
          <dt className="text-muted-foreground">{t('summary.remaining')}</dt>
          <dd className="tabular-nums">{percent(team.remaining_percent)}</dd>
        </div>
      </dl>
      {!team.owner && parsePercent(team.remaining_percent) > 0 && (
        <p className="text-sm text-muted-foreground">
          {t('summary.unassigned', { percent: percent(team.remaining_percent) })}
        </p>
      )}
    </div>
  )
}

interface AddMemberFormProps {
  team: DealTeam
  roles: DealMemberRole[]
  busy: boolean
  onCancel: () => void
  onSubmit: (body: { user_id: string; role_id: string | null; credit_percent: string }, name: string) => Promise<void>
}

function AddMemberForm({ team, roles, busy, onCancel, onSubmit }: AddMemberFormProps) {
  const { t, i18n } = useTranslation('deals', { keyPrefix: 'team' })
  const formId = useId()
  const [userId, setUserId] = useState<string | null>(null)
  const users = useOrganizationUsers()
  const [roleId, setRoleId] = useState<string | null>(null)
  const [credit, setCredit] = useState('0')
  const [userError, setUserError] = useState(false)

  const submit = async () => {
    if (!userId) {
      setUserError(true)
      return
    }
    const name = users.data?.find((user) => user.id === userId)?.display_name ?? ''
    await onSubmit({ user_id: userId, role_id: roleId, credit_percent: toPercentInput(credit) || '0' }, name)
  }

  return (
    <form
      className="grid gap-3 rounded-md border p-3 @2xl:grid-cols-[minmax(0,1fr)_14rem_8rem] @2xl:items-end"
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
    >
      <Field id={`${formId}-user`} label={t('add.user')} className="min-w-0">
        <UserFieldInput
          id={`${formId}-user`}
          value={userId}
          clearable={false}
          invalid={userError}
          onChange={(value) => {
            setUserId(value)
            setUserError(false)
          }}
        />
        {userError && <SharedFieldError>{t('add.userRequired')}</SharedFieldError>}
      </Field>
      <Field label={t('add.role')}>
        {(fieldControl) => <RoleSelect id={fieldControl.id} value={roleId} roles={roles} onChange={setRoleId} />}
      </Field>
      <Field label={t('add.credit')}>
        {(fieldControl) => <Input placeholder={t('placeholders.credit')}
          id={fieldControl.id}
          inputMode="decimal"
          value={credit}
          className="tabular-nums"
          onChange={(event) => setCredit(event.target.value)}
        />}
      </Field>
      <div className="flex flex-wrap items-center justify-between gap-2 @2xl:col-span-3">
        <span className="text-xs text-muted-foreground">
          {t('add.available', { percent: formatPercent(team.remaining_percent, i18n.language) })}
        </span>
        <div className="flex gap-2">
          <Button type="button" size="sm" variant="ghost" onClick={onCancel}>
            {t('add.cancel')}
          </Button>
          <Button type="submit" size="sm" disabled={busy}>
            {t('add.submit')}
          </Button>
        </div>
      </div>
    </form>
  )
}
