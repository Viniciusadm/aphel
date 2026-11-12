import { CalendarPlus, Check, MoreHorizontal, UserRoundPen } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import type { Activity } from '@/sdk'
import { useActivityActions } from '../lib/useActivityActions'
import { ActivityRow } from './ActivityRow'
import { ReassignDialog, type ReassignTarget } from './ReassignDialog'

export function ActivityListItem({ record }: { record: unknown }) {
  const { t: tl } = useTranslation('tasks', { keyPrefix: 'list' })
  const activity = record as Activity
  const actions = useActivityActions(activity)
  const [reassigning, setReassigning] = useState<ReassignTarget | null>(null)
  const open = activity.status === 'open'
  const { pending } = actions

  return (
    <>
      <ActivityRow
        activity={activity}
        onToggle={(_, done) => actions.setDone(done)}
        pending={pending}
        showRecord
        actions={
          open && (
            <>
              <div className="hidden items-center gap-1 md:flex">
                <Button variant="ghost" size="sm" disabled={pending} onClick={() => actions.postpone(1)}>
                  {tl('actions.postponeDay')}
                </Button>
                <Button variant="ghost" size="sm" disabled={pending} onClick={() => actions.postpone(7)}>
                  {tl('actions.postponeWeek')}
                </Button>
              </div>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="icon" disabled={pending} aria-label={tl('actions.menu', { title: activity.title })}>
                    <MoreHorizontal aria-hidden="true" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem onSelect={() => actions.setDone(true)}>
                    <Check aria-hidden="true" />
                    {tl('actions.complete')}
                  </DropdownMenuItem>
                  <DropdownMenuItem onSelect={() => actions.postpone(1)}>
                    <CalendarPlus aria-hidden="true" />
                    {tl('actions.postponeDay')}
                  </DropdownMenuItem>
                  <DropdownMenuItem onSelect={() => actions.postpone(7)}>
                    <CalendarPlus aria-hidden="true" />
                    {tl('actions.postponeWeek')}
                  </DropdownMenuItem>
                  <DropdownMenuItem onSelect={() => setReassigning(activity)}>
                    <UserRoundPen aria-hidden="true" />
                    {tl('actions.reassign')}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </>
          )
        }
      />
      <ReassignDialog
        item={reassigning}
        onOpenChange={(value) => !value && setReassigning(null)}
        onConfirm={(_, ownerId) => {
          setReassigning(null)
          void actions.reassign(ownerId)
        }}
      />
    </>
  )
}
