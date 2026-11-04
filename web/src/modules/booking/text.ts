import type { ParseKeys } from 'i18next'
import { useTranslation } from 'react-i18next'

export type BookingKey = ParseKeys<'booking'>

export function useBookingText() {
  const { t } = useTranslation('booking')
  return (key: string) => t(key as BookingKey)
}
