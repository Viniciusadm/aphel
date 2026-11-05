import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { type PaletteCommand, useRegisterCommands } from '@/sdk'
import { useFavorites } from './api'
import { favoriteHref, favoriteIcon } from './links'

export function FavoriteCommands() {
  const { t } = useTranslation('favorites')
  const navigate = useNavigate()
  const favorites = useFavorites()
  const commands = useMemo<PaletteCommand[]>(
    () =>
      (favorites.data ?? []).flatMap((favorite) => {
        const href = favoriteHref(favorite)
        if (!href || !favorite.title) return []
        return [
          {
            id: `favorite:${favorite.id}`,
            label: favorite.title,
            group: 'favorites',
            icon: favoriteIcon(favorite),
            keywords: [t('commands.group'), favorite.record_type],
            run: () => void navigate(href),
          },
        ]
      }),
    [favorites.data, navigate, t],
  )
  useRegisterCommands(commands)
  return null
}
