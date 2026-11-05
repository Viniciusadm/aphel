import type { components } from '@/modules/telephony/api-types'
import type {Activity} from '@/sdk'

type Schemas = components['schemas']

export type { PageMeta } from '@/sdk'

export type CallDirection = Schemas['CallDirection']
export type CallSessionStatus = Schemas['CallSessionStatus']
export type CallSessionItemStatus = Schemas['CallSessionItemStatus']
export type CallOutcomeSummary = Schemas['CallOutcomeSummaryResponse']
export type ActivityCall = Schemas['ActivityCallResponse']
export type CallActivity = Activity
export type CallOutcome = Schemas['CallOutcomeResponse']
export type CreateCallOutcomeRequest = Schemas['CreateCallOutcomeRequest']
export type UpdateCallOutcomeRequest = Schemas['UpdateCallOutcomeRequest']
export type LogCallRequest = Schemas['LogCallRequest']
export type UpdateCallRequest = Schemas['UpdateCallRequest']
export type TelephonyProvider = Schemas['TelephonyProviderResponse']
export type StartCallRequest = Schemas['StartCallRequest']
export type StartedCall = Schemas['StartedCallResponse']
export type CallRecording = Schemas['CallRecordingResponse']
export type UserReference = Schemas['UserReferenceResponse']
export type CallReportOutcomeCount = Schemas['CallReportOutcomeCountResponse']
export type CallReportUser = Schemas['CallReportUserResponse']
export type CallReport = Schemas['CallReportResponse']
export type CallSessionCounts = Schemas['CallSessionCountsResponse']
export type CallSessionItem = Schemas['CallSessionItemResponse']
export type CallSession = Schemas['CallSessionResponse']
export type CallSessionPage = Schemas['Page_CallSessionResponse']
export type CallSessionItemPage = Schemas['Page_CallSessionItemResponse']
export type StartCallSessionRequest = Schemas['StartCallSessionRequest']
export type SessionCallRequest = Schemas['SessionCallRequest']
export type SessionCallResult = Schemas['SessionCallResponse']

export interface CallReportQuery {
  from?: string
  to?: string
  owner_id?: string
}
