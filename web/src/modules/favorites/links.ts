import { ListFilter, type LucideIcon } from 'lucide-react'
import { recordDetailHref, recordTypeIcon, viewHref } from '@/sdk'
import type { Favorite } from './api'

export function favoriteHref(favorite: Favorite): string | undefined {
  if (!favorite.available) return undefined
  if (favorite.favorite_type === 'record' && favorite.record_id) {
    return recordDetailHref(favorite.record_type, favorite.record_id)
  }
  if (favorite.favorite_type === 'view' && favorite.view_id) return viewHref(favorite.record_type, favorite.view_id)
  return undefined
}

export function favoriteIcon(favorite: Favorite): LucideIcon {
  return favorite.favorite_type === 'view' ? ListFilter : recordTypeIcon(favorite.record_type)
}
