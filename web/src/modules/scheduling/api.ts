import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api, isApiError } from '@/api'
import type {
  BookingFilters,
  MeetingBookingResponse,
  MeetingTypeRequest,
  MeetingTypeResponse,
  Page,
  SchedulingAvailabilityRequest,
  SchedulingAvailabilityResponse,
  SchedulingContactFieldResponse,
  SchedulingReference,
} from './types'

export const MEETING_TYPES_PER_PAGE = 100
export const BOOKINGS_PER_PAGE = 25

const id = encodeURIComponent

function ifMatch(version?: string | null): HeadersInit | undefined {
  return version ? { 'If-Match': version } : undefined
}

export const schedulingKeys = {
  all: ['scheduling'] as const,
  availability: (userId: string | null) => [...schedulingKeys.all, 'availability', userId] as const,
  contactFields: () => [...schedulingKeys.all, 'contact-fields'] as const,
  distributionRules: () => [...schedulingKeys.all, 'distribution-rules'] as const,
  meetingTypes: () => [...schedulingKeys.all, 'meeting-types'] as const,
  bookings: () => [...schedulingKeys.all, 'bookings'] as const,
  bookingList: (filters: BookingFilters) => [...schedulingKeys.bookings(), filters] as const,
}

export const schedulingApi = {
  availability: (userId: string | null) =>
    api.get<SchedulingAvailabilityResponse>('/v1/scheduling/availability', {
      query: { user_id: userId ?? undefined },
    }),
  saveAvailability: (userId: string | null, request: SchedulingAvailabilityRequest, version?: string | null) =>
    api.put<SchedulingAvailabilityResponse>('/v1/scheduling/availability', request, {
      query: { user_id: userId ?? undefined },
      headers: ifMatch(version),
    }),
  contactFields: () => api.get<SchedulingContactFieldResponse[]>('/v1/scheduling/contact-fields'),
  distributionRules: () => api.get<SchedulingReference[]>('/v1/scheduling/distribution-rules'),
  meetingTypes: () =>
    api.get<Page<MeetingTypeResponse>>('/v1/scheduling/meeting-types', {
      query: { per_page: MEETING_TYPES_PER_PAGE },
    }),
  createMeetingType: (request: MeetingTypeRequest) =>
    api.post<MeetingTypeResponse>('/v1/scheduling/meeting-types', request),
  updateMeetingType: (meetingTypeId: string, request: MeetingTypeRequest, version?: string | null) =>
    api.patch<MeetingTypeResponse>(`/v1/scheduling/meeting-types/${id(meetingTypeId)}`, request, {
      headers: ifMatch(version),
    }),
  removeMeetingType: (meetingTypeId: string) =>
    api.delete<void>(`/v1/scheduling/meeting-types/${id(meetingTypeId)}`),
  bookings: (filters: BookingFilters) =>
    api.get<Page<MeetingBookingResponse>>('/v1/scheduling/bookings', {
      query: {
        status: filters.status === 'all' ? undefined : filters.status,
        meeting_type_id: filters.meetingTypeId ?? undefined,
        upcoming: filters.upcoming ? true : undefined,
        page: filters.page,
        per_page: BOOKINGS_PER_PAGE,
      },
    }),
  cancelBooking: (bookingId: string, reason: string | null) =>
    api.post<MeetingBookingResponse>(`/v1/scheduling/bookings/${id(bookingId)}/cancel`, {
      reason: reason ?? undefined,
    }),
}

export function useAvailability(userId: string | null) {
  return useQuery({
    queryKey: schedulingKeys.availability(userId),
    queryFn: () => schedulingApi.availability(userId),
  })
}

export function useSaveAvailability(userId: string | null) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ request, version }: { request: SchedulingAvailabilityRequest; version: string | null }) =>
      schedulingApi.saveAvailability(userId, request, version),
    onSuccess: (availability) => {
      queryClient.setQueryData(schedulingKeys.availability(userId), availability)
    },
  })
}

export function useContactFields() {
  return useQuery({
    queryKey: schedulingKeys.contactFields(),
    queryFn: schedulingApi.contactFields,
    staleTime: 5 * 60_000,
  })
}

export function useDistributionRules() {
  return useQuery({
    queryKey: schedulingKeys.distributionRules(),
    queryFn: async () => {
      try {
        return await schedulingApi.distributionRules()
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
    queryKey: schedulingKeys.meetingTypes(),
    queryFn: schedulingApi.meetingTypes,
    select: (page) => page.data,
  })
}

export function useSaveMeetingType(meetingType: MeetingTypeResponse | null) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (request: MeetingTypeRequest) =>
      meetingType
        ? schedulingApi.updateMeetingType(meetingType.id, request, meetingType.updated_at)
        : schedulingApi.createMeetingType(request),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: schedulingKeys.meetingTypes() }),
  })
}

export function useRemoveMeetingType() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: schedulingApi.removeMeetingType,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: schedulingKeys.meetingTypes() }),
  })
}

export function useBookings(filters: BookingFilters) {
  return useQuery({
    queryKey: schedulingKeys.bookingList(filters),
    queryFn: () => schedulingApi.bookings(filters),
    placeholderData: keepPreviousData,
  })
}

export function useCancelBooking() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ bookingId, reason }: { bookingId: string; reason: string | null }) =>
      schedulingApi.cancelBooking(bookingId, reason),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: schedulingKeys.bookings() }),
  })
}
