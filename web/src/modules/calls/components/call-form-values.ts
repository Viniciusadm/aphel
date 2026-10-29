import type { CallDirection } from '../types'

export interface CallFormValues {
  direction: CallDirection
  outcomeKey: string
  minutes: string
  seconds: string
  notes: string
}

export const EMPTY_CALL_FORM: CallFormValues = {
  direction: 'outbound',
  outcomeKey: '',
  minutes: '',
  seconds: '',
  notes: '',
}
