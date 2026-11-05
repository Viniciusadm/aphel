import type { components } from '@/modules/booking/api-types'

type Schemas = components['schemas']

export type { RecordsPage as Page, PageMeta } from '@/sdk'

export type BookingWeeklyInterval = Schemas['BookingWeeklyInterval']
export type BookingAvailabilityRequest = Schemas['BookingAvailabilityRequest']
export type BookingAvailabilityResponse = Schemas['BookingAvailabilityResponse']
export type BookingContactFieldResponse = Schemas['BookingContactFieldResponse']
export type BookingReference = Schemas['BookingReference']
export type MeetingQuestionKind = Schemas['MeetingQuestionKind']
export type MeetingLocationKind = Schemas['MeetingLocationKind']
export type MeetingTypeKind = 'personal' | 'team'
export type MeetingBookingStatus = Schemas['MeetingBookingStatus']
export type MeetingQuestionRequest = Schemas['MeetingQuestionRequest']
export type MeetingQuestionResponse = Schemas['MeetingQuestionResponse']
export type MeetingTypeRequest = Schemas['MeetingTypeRequest']
export type MeetingTypeResponse = Omit<Schemas['MeetingTypeResponse'], 'kind'> & { kind: MeetingTypeKind }
export type MeetingBookingResponse = Schemas['MeetingBookingResponse']

export interface BookingFilters {
  status: MeetingBookingStatus | 'all'
  meetingTypeId: string | null
  upcoming: boolean
  page: number
}

export const LOCATION_KINDS: MeetingLocationKind[] = ['none', 'video', 'address', 'phone']
export const QUESTION_KINDS: MeetingQuestionKind[] = ['text', 'long_text']
export const WEEKDAYS = [1, 2, 3, 4, 5, 6, 7] as const
