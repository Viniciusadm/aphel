import { Trash2, User, Users } from 'lucide-react'
import { useId, useMemo, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { ErrorState } from '@/components/ui/error-state'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { useOrganizationUsers } from '../api'
import { NativeSelect } from '../filters/NativeSelect'
import { useViewTeams } from '../views/viewsApi'
import { useRecordShareMutations, useRecordShares } from './api'
import { sharingErrorMessage } from './errors'
import type { RecordShare, ShareAccess, ShareGranteeType } from './types'

const ACCESS_OPTIONS: ShareAccess[] = ['read', 'edit']
const GRANTEE_TYPES: ShareGranteeType[] = ['user', 'team']

export interface ShareRecordDialogProps {
  recordType: string
  recordId: string
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function ShareRecordDialog({ recordType, recordId, open, onOpenChange }: ShareRecordDialogProps) {
  const { t } = useTranslation('sharing')
  const shares = useRecordShares(recordType, recordId, open)
  const mutations = useRecordShareMutations(recordType, recordId)

  const fail = (error: unknown) =>
    toast({ title: t('toast.errorTitle'), description: sharingErrorMessage(error, t), variant: 'destructive' })

  let body
  if (shares.isPending) {
    body = (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-10 w-full" />
      </div>
    )
  } else if (shares.isError) {
    body = <ErrorState title={sharingErrorMessage(shares.error, t)} onRetry={() => void shares.refetch()} />
  } else {
    const data = shares.data
    body = (
      <div className="grid gap-4">
        {data.can_manage && (
          <ShareForm
            ownerId={data.owner_id ?? null}
            existing={data.shares}
            pending={mutations.create.isPending}
            onSubmit={(request) =>
              mutations.create.mutate(request, {
                onSuccess: () => toast({ title: t('toast.shared'), variant: 'success' }),
                onError: fail,
              })
            }
          />
        )}
        {data.shares.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t('dialog.empty')}</p>
        ) : (
          <ul className="grid gap-2" aria-label={t('dialog.listLabel')}>
            {data.shares.map((share) => (
              <ShareRow
                key={share.id}
                share={share}
                canManage={data.can_manage}
                pending={mutations.update.isPending || mutations.remove.isPending}
                onAccessChange={(access) => mutations.update.mutate({ share, access }, { onError: fail })}
                onRemove={() =>
                  mutations.remove.mutate(share, {
                    onSuccess: () => toast({ title: t('toast.removed'), variant: 'success' }),
                    onError: fail,
                  })
                }
              />
            ))}
          </ul>
        )}
        {!data.can_manage && <p className="text-xs text-muted-foreground">{t('dialog.readOnly')}</p>}
      </div>
    )
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>{t('dialog.title')}</DialogTitle>
          <DialogDescription>{t('dialog.description')}</DialogDescription>
        </DialogHeader>
        {body}
      </DialogContent>
    </Dialog>
  )
}

interface ShareFormProps {
  ownerId: string | null
  existing: RecordShare[]
  pending: boolean
  onSubmit: (request: { grantee_type: ShareGranteeType; grantee_id: string; access: ShareAccess }) => void
}

function ShareForm({ ownerId, existing, pending, onSubmit }: ShareFormProps) {
  const { t } = useTranslation('sharing')
  const typeId = useId()
  const granteeId = useId()
  const accessId = useId()
  const [granteeType, setGranteeType] = useState<ShareGranteeType>('user')
  const [grantee, setGrantee] = useState('')
  const [access, setAccess] = useState<ShareAccess>('read')
  const users = useOrganizationUsers()
  const teams = useViewTeams(granteeType === 'team')

  const options = useMemo(() => {
    const taken = new Set(existing.filter((share) => share.grantee.type === granteeType).map((share) => share.grantee.id))
    if (granteeType === 'user') {
      return (users.data ?? [])
        .filter((user) => user.id !== ownerId && !taken.has(user.id))
        .map((user) => ({ id: user.id, name: user.display_name }))
    }
    return (teams.data ?? []).filter((team) => !taken.has(team.id))
  }, [existing, granteeType, ownerId, teams.data, users.data])

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (!grantee) return
    onSubmit({ grantee_type: granteeType, grantee_id: grantee, access })
    setGrantee('')
  }

  return (
    <form className="grid gap-3 rounded-md border p-3" onSubmit={submit}>
      <div className="grid gap-3 sm:grid-cols-[8rem_1fr_8rem]">
        <div className="grid gap-2.5">
          <Label htmlFor={typeId}>{t('form.granteeType')}</Label>
          <NativeSelect
            id={typeId}
            value={granteeType}
            onChange={(event) => {
              setGranteeType(event.target.value as ShareGranteeType)
              setGrantee('')
            }}
          >
            {GRANTEE_TYPES.map((type) => (
              <option key={type} value={type}>
                {t(`granteeTypes.${type}`)}
              </option>
            ))}
          </NativeSelect>
        </div>
        <div className="grid min-w-0 gap-2.5">
          <Label htmlFor={granteeId}>{t(granteeType === 'user' ? 'form.user' : 'form.team')}</Label>
          <NativeSelect id={granteeId} value={grantee} onChange={(event) => setGrantee(event.target.value)}>
            <option value="">{t('form.choose')}</option>
            {options.map((option) => (
              <option key={option.id} value={option.id}>
                {option.name}
              </option>
            ))}
          </NativeSelect>
          {granteeType === 'team' && teams.isError && (
            <p className="text-xs text-muted-foreground">{t('form.teamsUnavailable')}</p>
          )}
        </div>
        <div className="grid gap-2.5">
          <Label htmlFor={accessId}>{t('form.access')}</Label>
          <NativeSelect id={accessId} value={access} onChange={(event) => setAccess(event.target.value as ShareAccess)}>
            {ACCESS_OPTIONS.map((option) => (
              <option key={option} value={option}>
                {t(`access.${option}`)}
              </option>
            ))}
          </NativeSelect>
        </div>
      </div>
      <div className="flex items-center justify-between gap-2">
        <p className="text-xs text-muted-foreground">{t(`accessHelp.${access}`)}</p>
        <Button type="submit" size="sm" disabled={!grantee || pending}>
          {t('form.submit')}
        </Button>
      </div>
    </form>
  )
}

interface ShareRowProps {
  share: RecordShare
  canManage: boolean
  pending: boolean
  onAccessChange: (access: ShareAccess) => void
  onRemove: () => void
}

function ShareRow({ share, canManage, pending, onAccessChange, onRemove }: ShareRowProps) {
  const { t } = useTranslation('sharing')
  const Icon = share.grantee.type === 'team' ? Users : User
  const name = share.grantee.name ?? t('unknownGrantee')
  return (
    <li className="flex items-center gap-3 rounded-md border px-3 py-2">
      <Icon className="size-4 shrink-0 text-muted-foreground" aria-hidden />
      <div className="grid min-w-0 flex-1">
        <span className="truncate text-sm font-medium">{name}</span>
        <span className="truncate text-xs text-muted-foreground">
          {t(`granteeTypes.${share.grantee.type}`)}
          {share.shared_by ? ` · ${t('sharedBy', { name: share.shared_by.display_name })}` : ''}
        </span>
      </div>
      {canManage ? (
        <>
          <NativeSelect
            className="w-32"
            aria-label={t('form.accessFor', { name })}
            value={share.access}
            disabled={pending}
            onChange={(event) => onAccessChange(event.target.value as ShareAccess)}
          >
            {ACCESS_OPTIONS.map((option) => (
              <option key={option} value={option}>
                {t(`access.${option}`)}
              </option>
            ))}
          </NativeSelect>
          <Button
            variant="ghost"
            size="icon"
            disabled={pending}
            onClick={onRemove}
            aria-label={t('form.remove', { name })}
          >
            <Trash2 aria-hidden />
          </Button>
        </>
      ) : (
        <span className="text-xs text-muted-foreground">{t(`access.${share.access}`)}</span>
      )}
    </li>
  )
}
