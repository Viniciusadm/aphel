import { Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Avatar } from '@/components/ui/avatar'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { formatPercent, parsePercent, toPercentInput } from './errors'
import { RoleSelect } from './RoleSelect'
import type { DealMember, DealMemberRole, UpdateDealMemberRequest } from './types'

export const MEMBER_GRID_CLASS = '@2xl:grid-cols-[minmax(0,1fr)_14rem_8rem_2.5rem] @2xl:items-center'

export interface MemberRowProps {
  member: DealMember
  isOwner: boolean
  roles: DealMemberRole[]
  readOnly: boolean
  busy: boolean
  onSave: (member: DealMember, body: UpdateDealMemberRequest) => Promise<void>
  onRemove: (member: DealMember) => void
}

export function MemberRow({ member, isOwner, roles, readOnly, busy, onSave, onRemove }: MemberRowProps) {
  const { t, i18n } = useTranslation('deals', { keyPrefix: 'team' })
  const [credit, setCredit] = useState(member.credit_percent)
  const name = member.user.display_name

  const saveCredit = async () => {
    const next = toPercentInput(credit)
    if (next === '' || parsePercent(next) === parsePercent(member.credit_percent)) {
      setCredit(member.credit_percent)
      return
    }
    try {
      await onSave(member, { credit_percent: next })
    } catch {
      setCredit(member.credit_percent)
    }
  }

  return (
    <li
      className={`grid gap-2 rounded-md border p-3 @2xl:gap-3 @2xl:rounded-none @2xl:border-0 @2xl:border-b @2xl:px-2 @2xl:py-2 ${MEMBER_GRID_CLASS}`}
    >
      <div className="flex min-w-0 items-center gap-2">
        <Avatar name={name} />
        <span className="truncate text-sm font-medium">{name}</span>
        {isOwner && <Badge variant="secondary">{t('ownerBadge')}</Badge>}
        {!member.user_active && <Badge variant="outline">{t('inactiveUser')}</Badge>}
      </div>
      {readOnly ? (
        <span className="text-sm text-muted-foreground">{member.role?.name ?? t('noRole')}</span>
      ) : (
        <RoleSelect
          value={member.role?.id ?? null}
          roles={roles}
          current={member.role}
          disabled={busy}
          aria-label={t('roleLabel', { name })}
          onChange={(roleId) => void onSave(member, { role_id: roleId }).catch(() => undefined)}
        />
      )}
      {readOnly ? (
        <span className="text-sm tabular-nums @2xl:text-right">
          {formatPercent(member.credit_percent, i18n.language)}
        </span>
      ) : (
        <Input placeholder={t('placeholders.credit')}
          inputMode="decimal"
          value={credit}
          disabled={busy}
          aria-label={t('creditLabel', { name })}
          className="tabular-nums @2xl:text-right"
          onChange={(event) => setCredit(event.target.value)}
          onBlur={() => void saveCredit()}
          onKeyDown={(event) => {
            if (event.key === 'Enter') {
              event.preventDefault()
              event.currentTarget.blur()
            }
            if (event.key === 'Escape') setCredit(member.credit_percent)
          }}
        />
      )}
      {readOnly ? (
        <span />
      ) : (
        <Button
          size="icon"
          variant="ghost"
          disabled={busy}
          aria-label={t('removeLabel', { name })}
          onClick={() => onRemove(member)}
          className="justify-self-end"
        >
          <Trash2 aria-hidden="true" />
        </Button>
      )}
    </li>
  )
}
