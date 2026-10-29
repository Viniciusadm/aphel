import type { components } from '@/api'

type Schemas = components['schemas']

export type { Page, PageMeta } from '@/features/records/types'

export type SchedulingWeeklyInterval = Schemas['SchedulingWeeklyInterval']
export type SchedulingAvailabilityRequest = Schemas['SchedulingAvailabilityRequest']
export type SchedulingAvailabilityResponse = Schemas['SchedulingAvailabilityResponse']
export type SchedulingContactFieldResponse = Schemas['SchedulingContactFieldResponse']
export type SchedulingReference = Schemas['SchedulingReference']
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
