import { CalendarClock, Copy, ExternalLink, Pencil, Plus, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useMeetingTypes, useRemoveMeetingType } from '../api'
import { publicBookingUrl } from '../lib/format'
import type { MeetingTypeResponse } from '../types'
import { MeetingTypeDialog } from './MeetingTypeDialog'

type DialogState = { mode: 'create' } | { mode: 'edit'; meetingType: MeetingTypeResponse }

interface MeetingTypeCardProps {
  meetingType: MeetingTypeResponse
  onEdit: () => void
  onRemove: () => void
}

function MeetingTypeCard({ meetingType, onEdit, onRemove }: MeetingTypeCardProps) {
  const { t } = useTranslation('booking')
  const url = publicBookingUrl(meetingType.booking_path)

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(url)
      notifySuccess(t('meetingTypes.linkCopied'))
    } catch (error) {
      notifyError(error)
    }
  }

  return (
    <li className="flex flex-col gap-3 rounded-lg border bg-card p-4">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="grid min-w-0 gap-1">
          <p className="truncate font-medium">{meetingType.name}</p>
          <p className="truncate text-xs text-muted-foreground">{url}</p>
        </div>
        <div className="flex flex-wrap gap-1">
          <Badge variant="outline">{t(`kinds.${meetingType.kind}`)}</Badge>
          <Badge variant={meetingType.is_active ? 'success' : 'secondary'}>
            {t(meetingType.is_active ? 'meetingTypes.active' : 'meetingTypes.inactive')}
          </Badge>
        </div>
      </div>
      <dl className="grid grid-cols-2 gap-2 text-sm">
        <div>
          <dt className="text-xs text-muted-foreground">{t('meetingTypes.duration')}</dt>
          <dd>{t('meetingTypes.minutes', { count: meetingType.duration_minutes })}</dd>
        </div>
        <div>
          <dt className="text-xs text-muted-foreground">
            {t(meetingType.kind === 'team' ? 'meetingTypes.distributionRule' : 'meetingTypes.owner')}
          </dt>
          <dd className="truncate">
            {meetingType.kind === 'team' ? (meetingType.distribution_rule?.name ?? '—') : meetingType.owner.name}
          </dd>
        </div>
      </dl>
      <div className="mt-auto flex flex-wrap gap-1">
        <Button variant="outline" size="sm" onClick={() => void copy()}>
          <Copy />
          {t('meetingTypes.copyLink')}
        </Button>
        <Button variant="ghost" size="sm" asChild>
          <a href={url} target="_blank" rel="noreferrer">
            <ExternalLink />
            {t('meetingTypes.openLink')}
          </a>
        </Button>
        <Button variant="ghost" size="icon" aria-label={t('meetingTypes.edit')} onClick={onEdit}>
          <Pencil />
        </Button>
        <Button variant="ghost" size="icon" aria-label={t('meetingTypes.remove')} onClick={onRemove}>
          <Trash2 />
        </Button>
      </div>
    </li>
  )
}

export function MeetingTypesPanel() {
  const { t } = useTranslation('booking')
  const meetingTypes = useMeetingTypes()
  const remove = useRemoveMeetingType()
  const [dialog, setDialog] = useState<DialogState | null>(null)
  const [removing, setRemoving] = useState<MeetingTypeResponse | null>(null)

  const createButton = (
    <Button onClick={() => setDialog({ mode: 'create' })}>
      <Plus />
      {t('meetingTypes.create')}
    </Button>
  )

  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-sm text-muted-foreground">{t('meetingTypes.description')}</p>
        {createButton}
      </div>
      {meetingTypes.isPending ? (
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          <Skeleton className="h-40" />
          <Skeleton className="h-40" />
        </div>
      ) : meetingTypes.isError ? (
        <ErrorState onRetry={() => void meetingTypes.refetch()} />
      ) : meetingTypes.data.length === 0 ? (
        <EmptyState
          icon={CalendarClock}
          title={t('meetingTypes.emptyTitle')}
          description={t('meetingTypes.emptyDescription')}
          action={createButton}
        />
      ) : (
        <ul className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {meetingTypes.data.map((meetingType) => (
            <MeetingTypeCard
              key={meetingType.id}
              meetingType={meetingType}
              onEdit={() => setDialog({ mode: 'edit', meetingType })}
              onRemove={() => setRemoving(meetingType)}
            />
          ))}
        </ul>
      )}
      {dialog && (
        <MeetingTypeDialog
          meetingType={dialog.mode === 'edit' ? dialog.meetingType : null}
          onClose={() => setDialog(null)}
        />
      )}
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => !open && setRemoving(null)}
        variant="destructive"
        title={t('meetingTypes.removeTitle')}
        description={t('meetingTypes.removeDescription', { name: removing?.name ?? '' })}
        onConfirm={async () => {
          if (!removing) return
          await remove.mutateAsync(removing.id)
          notifySuccess(t('meetingTypes.toast.removed'))
        }}
      />
    </div>
  )
}
