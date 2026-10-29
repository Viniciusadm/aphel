import type { components } from '@/api'

type Schemas = components['schemas']

export type PublicMeetingQuestion = Schemas['PublicMeetingQuestion']
export type PublicMeetingTypeResponse = Schemas['PublicMeetingTypeResponse']
export type PublicSlot = Schemas['PublicSlot']
export type PublicSlotsResponse = Schemas['PublicSlotsResponse']
export type PublicBookingRequest = Omit<Schemas['PublicBookingRequest'], 'answers'> & { answers: Record<string, string> }
export type PublicBookingResponse = Schemas['PublicBookingResponse']

export interface SlotRange {
  from: string
  to: string
}
