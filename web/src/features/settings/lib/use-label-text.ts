import { useTranslation } from 'react-i18next'
import { useCustomObjectsAdmin } from '../objects/api'
import type { PermissionLabel } from './permissions'

export function useLabelText() {
  const { t } = useTranslation('settings')
  const objects = useCustomObjectsAdmin()
  const customLabel = (fallback: string) =>
    objects.data?.find((object) => object.record_type === fallback)?.label_plural ?? fallback
  return (label: PermissionLabel) => (label.key ? t(label.key) : customLabel(label.fallback))
}
