import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api, isApiError } from '@/api'
import type {
  BookingFilters,
  MeetingBookingResponse,
  MeetingTypeRequest,
  MeetingTypeResponse,
  Page,
  BookingAvailabilityRequest,
  BookingAvailabilityResponse,
  BookingContactFieldResponse,
  BookingReference,
} from './types'

export const MEETING_TYPES_PER_PAGE = 100
export const BOOKINGS_PER_PAGE = 25

const id = encodeURIComponent

function ifMatch(version?: string | null): HeadersInit | undefined {
  return version ? { 'If-Match': version } : undefined
}

export const bookingKeys = {
  all: ['booking'] as const,
  availability: (userId: string | null) => [...bookingKeys.all, 'availability', userId] as const,
  contactFields: () => [...bookingKeys.all, 'contact-fields'] as const,
  distributionRules: () => [...bookingKeys.all, 'distribution-rules'] as const,
  meetingTypes: () => [...bookingKeys.all, 'meeting-types'] as const,
  bookings: () => [...bookingKeys.all, 'bookings'] as const,
  bookingList: (filters: BookingFilters) => [...bookingKeys.bookings(), filters] as const,
}

export const bookingApi = {
  availability: (userId: string | null) =>
    api.get<BookingAvailabilityResponse>('/v1/booking/availability', {
      query: { user_id: userId ?? undefined },
    }),
  saveAvailability: (userId: string | null, request: BookingAvailabilityRequest, version?: string | null) =>
    api.put<BookingAvailabilityResponse>('/v1/booking/availability', request, {
      query: { user_id: userId ?? undefined },
      headers: ifMatch(version),
    }),
  contactFields: () => api.get<BookingContactFieldResponse[]>('/v1/booking/contact-fields'),
  distributionRules: () => api.get<BookingReference[]>('/v1/booking/distribution-rules'),
  meetingTypes: () =>
    api.get<Page<MeetingTypeResponse>>('/v1/booking/meeting-types', {
      query: { per_page: MEETING_TYPES_PER_PAGE },
    }),
  createMeetingType: (request: MeetingTypeRequest) =>
    api.post<MeetingTypeResponse>('/v1/booking/meeting-types', request),
  updateMeetingType: (meetingTypeId: string, request: MeetingTypeRequest, version?: string | null) =>
    api.patch<MeetingTypeResponse>(`/v1/booking/meeting-types/${id(meetingTypeId)}`, request, {
      headers: ifMatch(version),
    }),
  removeMeetingType: (meetingTypeId: string) =>
    api.delete<void>(`/v1/booking/meeting-types/${id(meetingTypeId)}`),
  bookings: (filters: BookingFilters) =>
    api.get<Page<MeetingBookingResponse>>('/v1/booking/bookings', {
      query: {
        status: filters.status === 'all' ? undefined : filters.status,
        meeting_type_id: filters.meetingTypeId ?? undefined,
        upcoming: filters.upcoming ? true : undefined,
        page: filters.page,
        per_page: BOOKINGS_PER_PAGE,
      },
    }),
  cancelBooking: (bookingId: string, reason: string | null) =>
    api.post<MeetingBookingResponse>(`/v1/booking/bookings/${id(bookingId)}/cancel`, {
      reason: reason ?? undefined,
    }),
}

export function useAvailability(userId: string | null) {
  return useQuery({
    queryKey: bookingKeys.availability(userId),
    queryFn: () => bookingApi.availability(userId),
  })
}

export function useSaveAvailability(userId: string | null) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ request, version }: { request: BookingAvailabilityRequest; version: string | null }) =>
      bookingApi.saveAvailability(userId, request, version),
    onSuccess: (availability) => {
      queryClient.setQueryData(bookingKeys.availability(userId), availability)
    },
  })
}

export function useContactFields() {
  return useQuery({
    queryKey: bookingKeys.contactFields(),
    queryFn: bookingApi.contactFields,
    staleTime: 5 * 60_000,
  })
}

export function useDistributionRules() {
  return useQuery({
    queryKey: bookingKeys.distributionRules(),
    queryFn: async () => {
      try {
        return await bookingApi.distributionRules()
      } catch (error) {
        if (isApiError(error) && error.status === 403) return null
        throw error
      }
    },
    staleTime: 5 * 60_000,
  })
}

export function useMeetingTypes() {
  return useQuery({
    queryKey: bookingKeys.meetingTypes(),
    queryFn: bookingApi.meetingTypes,
    select: (page) => page.data,
  })
}

export function useSaveMeetingType(meetingType: MeetingTypeResponse | null) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (request: MeetingTypeRequest) =>
      meetingType
        ? bookingApi.updateMeetingType(meetingType.id, request, meetingType.updated_at)
        : bookingApi.createMeetingType(request),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: bookingKeys.meetingTypes() }),
  })
}

export function useRemoveMeetingType() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: bookingApi.removeMeetingType,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: bookingKeys.meetingTypes() }),
  })
}

export function useBookings(filters: BookingFilters) {
  return useQuery({
    queryKey: bookingKeys.bookingList(filters),
    queryFn: () => bookingApi.bookings(filters),
    placeholderData: keepPreviousData,
  })
}

export function useCancelBooking() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ bookingId, reason }: { bookingId: string; reason: string | null }) =>
      bookingApi.cancelBooking(bookingId, reason),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: bookingKeys.bookings() }),
  })
}
