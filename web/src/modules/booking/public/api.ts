import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type {
  PublicBookingRequest,
  PublicBookingResponse,
  PublicMeetingTypeResponse,
  PublicSlotsResponse,
  SlotRange,
} from './types'

const id = encodeURIComponent

export const bookingKeys = {
  all: ['booking'] as const,
  meetingType: (slug: string) => [...bookingKeys.all, 'meeting-type', slug] as const,
  slots: (owner: string) => [...bookingKeys.all, 'slots', owner] as const,
  slotRange: (owner: string, range: SlotRange) => [...bookingKeys.slots(owner), range.from, range.to] as const,
  manage: (token: string) => [...bookingKeys.all, 'manage', token] as const,
}

export const bookingApi = {
  meetingType: (slug: string) => api.get<PublicMeetingTypeResponse>(`/v1/public/book/${id(slug)}`),
  slots: (slug: string, range: SlotRange) =>
    api.get<PublicSlotsResponse>(`/v1/public/book/${id(slug)}/slots`, { query: { ...range } }),
  book: (slug: string, request: PublicBookingRequest) =>
    api.post<PublicBookingResponse>(`/v1/public/book/${id(slug)}`, request),
  booking: (token: string) => api.get<PublicBookingResponse>(`/v1/public/book/manage/${id(token)}`),
  manageSlots: (token: string, range: SlotRange) =>
    api.get<PublicSlotsResponse>(`/v1/public/book/manage/${id(token)}/slots`, { query: { ...range } }),
  reschedule: (token: string, startsAt: string) =>
    api.post<PublicBookingResponse>(`/v1/public/book/manage/${id(token)}/reschedule`, { starts_at: startsAt }),
  cancel: (token: string, reason: string | null) =>
    api.post<PublicBookingResponse>(`/v1/public/book/manage/${id(token)}/cancel`, { reason: reason ?? undefined }),
}

export function usePublicMeetingType(slug: string) {
  return useQuery({ queryKey: bookingKeys.meetingType(slug), queryFn: () => bookingApi.meetingType(slug) })
}

export function usePublicBooking(token: string) {
  return useQuery({ queryKey: bookingKeys.manage(token), queryFn: () => bookingApi.booking(token) })
}

export function useBook(slug: string) {
  return useMutation({ mutationFn: (request: PublicBookingRequest) => bookingApi.book(slug, request) })
}

export function useReschedule(token: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (startsAt: string) => bookingApi.reschedule(token, startsAt),
    onSuccess: (booking) => {
      queryClient.setQueryData(bookingKeys.manage(token), booking)
      void queryClient.invalidateQueries({ queryKey: bookingKeys.slots(`manage:${token}`) })
    },
  })
}

export function useCancelPublicBooking(token: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (reason: string | null) => bookingApi.cancel(token, reason),
    onSuccess: (booking) => queryClient.setQueryData(bookingKeys.manage(token), booking),
  })
}
