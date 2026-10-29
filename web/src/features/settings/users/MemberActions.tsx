import { MoreHorizontal } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import type { Member } from '../api'

export type MemberAction = 'changeRole' | 'resend' | 'cancelInvitation' | 'resetPassword' | 'suspend' | 'reactivate'

interface MemberActionsProps {
  member: Member
  onAction: (action: MemberAction, member: Member) => void
}

export function MemberActions({ member, onAction }: MemberActionsProps) {
  const { t } = useTranslation('settings')
  const invited = member.status === 'invited'

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" aria-label={`${t('actions.more')}: ${member.display_name}`}>
          <MoreHorizontal />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onSelect={() => onAction('changeRole', member)}>{t('users.menu.changeRole')}</DropdownMenuItem>
        {invited && (
          <>
            <DropdownMenuItem onSelect={() => onAction('resend', member)}>{t('users.menu.resend')}</DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive" onSelect={() => onAction('cancelInvitation', member)}>
              {t('users.menu.cancelInvitation')}
            </DropdownMenuItem>
          </>
        )}
        {member.status === 'active' && (
          <>
            <DropdownMenuItem onSelect={() => onAction('resetPassword', member)}>
              {t('users.menu.resetPassword')}
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={() => onAction('suspend', member)}>{t('users.menu.suspend')}</DropdownMenuItem>
          </>
        )}
        {member.status === 'deactivated' && (
          <DropdownMenuItem onSelect={() => onAction('reactivate', member)}>
            {t('users.menu.reactivate')}
          </DropdownMenuItem>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
