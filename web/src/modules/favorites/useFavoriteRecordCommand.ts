import { Star, StarOff } from 'lucide-react'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useRegisterCommands } from '@/features/command-palette/context'
import type { PaletteCommand } from '@/features/command-palette/types'
import { useFavoriteToggle, type FavoriteTarget } from './api'

export function useFavoriteRecordCommand(target: FavoriteTarget) {
  const { t } = useTranslation('favorites')
  const { isFavorite, ready, toggle } = useFavoriteToggle(target)
  const commands = useMemo<PaletteCommand[]>(
    () =>
      ready
        ? [
            {
              id: 'record:favorite',
              label: isFavorite ? t('commands.unfavorite') : t('commands.favorite'),
              group: 'context',
              icon: isFavorite ? StarOff : Star,
              run: toggle,
            },
          ]
        : [],
    [isFavorite, ready, t, toggle],
  )
  useRegisterCommands(commands)
}
