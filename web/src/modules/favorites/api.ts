import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'
import { api, isApiError } from '@/api'
import type { components } from '@/api/schema'
import { toast } from '@/components/ui/toast-store'
import i18n from '@/i18n'

export type Favorite = components['schemas']['FavoriteResponse']
export type FavoriteType = components['schemas']['FavoriteType']
export type CreateFavoriteRequest = components['schemas']['CreateFavoriteRequest']

export const FAVORITES_PAGE_SIZE = 100
export const MAX_FAVORITES = 50

export type FavoriteTarget =
  | { favoriteType: 'record'; recordType: string; recordId: string }
  | { favoriteType: 'view'; recordType: string; viewId: string }

export const favoritesKeys = {
  all: ['favorites'] as const,
  list: () => [...favoritesKeys.all, 'list'] as const,
}

export const favoritesApi = {
  list: () =>
    api
      .get<{ data: Favorite[] }>('/v1/favorites', { query: { per_page: FAVORITES_PAGE_SIZE } })
      .then((page) => page.data),
  create: (request: CreateFavoriteRequest) => api.post<Favorite>('/v1/favorites', request),
  remove: (id: string) => api.delete<void>(`/v1/favorites/${encodeURIComponent(id)}`),
  reorder: (ids: string[]) => api.put<Favorite[]>('/v1/favorites/order', { ids }),
}

export function targetRequest(target: FavoriteTarget): CreateFavoriteRequest {
  return target.favoriteType === 'record'
    ? { favorite_type: 'record', record_type: target.recordType, record_id: target.recordId }
    : { favorite_type: 'view', view_id: target.viewId }
}

export function matchesTarget(favorite: Favorite, target: FavoriteTarget): boolean {
  if (favorite.favorite_type !== target.favoriteType) return false
  return target.favoriteType === 'record'
    ? favorite.record_type === target.recordType && favorite.record_id === target.recordId
    : favorite.view_id === target.viewId
}

export function useFavorites(enabled = true) {
  return useQuery({ queryKey: favoritesKeys.list(), queryFn: favoritesApi.list, enabled, staleTime: 60_000 })
}

function favoriteErrorTitle(error: unknown): string {
  if (isApiError(error) && error.code === 'favorite_limit_reached') return i18n.t('toast.limit', { ns: 'favorites' })
  return i18n.t('toast.error', { ns: 'favorites' })
}

export function useFavoriteMutations() {
  const queryClient = useQueryClient()
  const key = favoritesKeys.list()

  const add = useMutation({
    mutationFn: (target: FavoriteTarget) => favoritesApi.create(targetRequest(target)),
    onSuccess: (favorite) => {
      queryClient.setQueryData<Favorite[]>(key, (current) => (current ? [...current, favorite] : [favorite]))
      toast({ title: i18n.t('toast.added', { ns: 'favorites' }), variant: 'success' })
    },
    onError: (error) => {
      if (isApiError(error) && error.code === 'favorite_already_exists') {
        void queryClient.invalidateQueries({ queryKey: key })
        return
      }
      toast({ title: favoriteErrorTitle(error), variant: 'destructive' })
    },
  })

  const remove = useMutation({
    mutationFn: (favorite: Favorite) => favoritesApi.remove(favorite.id),
    onMutate: async (favorite) => {
      await queryClient.cancelQueries({ queryKey: key })
      const previous = queryClient.getQueryData<Favorite[]>(key)
      queryClient.setQueryData<Favorite[]>(key, (current) => current?.filter((item) => item.id !== favorite.id))
      return { previous }
    },
    onSuccess: () => toast({ title: i18n.t('toast.removed', { ns: 'favorites' }) }),
    onError: (error, _favorite, context) => {
      if (isApiError(error) && error.code === 'favorite_not_found') return
      if (context?.previous) queryClient.setQueryData(key, context.previous)
      toast({ title: favoriteErrorTitle(error), variant: 'destructive' })
    },
    onSettled: () => void queryClient.invalidateQueries({ queryKey: key }),
  })

  const reorder = useMutation({
    mutationFn: (ids: string[]) => favoritesApi.reorder(ids),
    onMutate: async (ids) => {
      await queryClient.cancelQueries({ queryKey: key })
      const previous = queryClient.getQueryData<Favorite[]>(key)
      if (previous) {
        const byId = new Map(previous.map((item) => [item.id, item]))
        queryClient.setQueryData<Favorite[]>(
          key,
          ids.flatMap((id) => {
            const item = byId.get(id)
            return item ? [item] : []
          }),
        )
      }
      return { previous }
    },
    onSuccess: (favorites) => queryClient.setQueryData(key, favorites),
    onError: (_error, _ids, context) => {
      if (context?.previous) queryClient.setQueryData(key, context.previous)
      toast({ title: i18n.t('toast.reorderError', { ns: 'favorites' }), variant: 'destructive' })
      void queryClient.invalidateQueries({ queryKey: key })
    },
  })

  return { add, remove, reorder }
}

export function useFavoriteToggle(target: FavoriteTarget | null) {
  const favorites = useFavorites(Boolean(target))
  const { add, remove } = useFavoriteMutations()
  const current = target ? favorites.data?.find((favorite) => matchesTarget(favorite, target)) : undefined
  const pending = add.isPending || remove.isPending
  const addFavorite = add.mutate
  const removeFavorite = remove.mutate

  const toggle = useCallback(() => {
    if (!target || pending) return
    if (current) removeFavorite(current)
    else addFavorite(target)
  }, [target, pending, current, addFavorite, removeFavorite])

  return { isFavorite: Boolean(current), ready: favorites.isSuccess, pending, toggle }
}
