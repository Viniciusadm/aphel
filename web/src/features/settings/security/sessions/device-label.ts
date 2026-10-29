import { useTranslation } from 'react-i18next'
import type { SecurityDevice } from './api'

export function useDeviceLabel() {
  const { t } = useTranslation('security')
  return (device: SecurityDevice) => {
    if (device.browser && device.os) return t('device.browserOnOs', { browser: device.browser, os: device.os })
    return device.browser ?? device.os ?? t('device.unknown')
  }
}
