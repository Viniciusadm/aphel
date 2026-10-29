import { Star } from 'lucide-react'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'
import { useFavoriteToggle, type FavoriteTarget } from './api'
import { useFavoriteRecordCommand } from './useFavoriteRecordCommand'

interface FavoriteToggleButtonProps {
  target: FavoriteTarget
  addLabel: string
  removeLabel: string
  className?: string
}

function FavoriteToggleButton({ target, addLabel, removeLabel, className }: FavoriteToggleButtonProps) {
  const { isFavorite, ready, pending, toggle } = useFavoriteToggle(target)
  const label = isFavorite ? removeLabel : addLabel

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className={cn('shrink-0', className)}
          onClick={toggle}
          disabled={!ready || pending}
          aria-pressed={isFavorite}
          aria-label={label}
        >
          <Star className={cn(isFavorite && 'fill-amber-400 text-amber-600 dark:text-amber-400')} aria-hidden />
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  )
}

export interface FavoriteRecordButtonProps {
  recordType: string
  recordId: string
  className?: string
}

export function FavoriteRecordButton({ recordType, recordId, className }: FavoriteRecordButtonProps) {
  const { t } = useTranslation('favorites')
  const target = useMemo<FavoriteTarget>(
    () => ({ favoriteType: 'record', recordType, recordId }),
    [recordType, recordId],
  )
  useFavoriteRecordCommand(target)
  return (
    <FavoriteToggleButton
      target={target}
      addLabel={t('button.add')}
      removeLabel={t('button.remove')}
      className={className}
    />
  )
}

export interface FavoriteViewButtonProps {
  recordType: string
  viewId: string
  className?: string
}

export function FavoriteViewButton({ recordType, viewId, className }: FavoriteViewButtonProps) {
  const { t } = useTranslation('favorites')
  const target = useMemo<FavoriteTarget>(() => ({ favoriteType: 'view', recordType, viewId }), [recordType, viewId])
  return (
    <FavoriteToggleButton
      target={target}
      addLabel={t('button.addView')}
      removeLabel={t('button.removeView')}
      className={className}
    />
  )
}
