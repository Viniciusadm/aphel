import { useTranslation } from 'react-i18next'
import { TabsTrigger } from '@/components/ui/tabs'
import { VERSIONS_TAB } from './tab'

export function VersionsTabTrigger() {
  const { t } = useTranslation('config-package-versioning')
  return <TabsTrigger value={VERSIONS_TAB}>{t('tab')}</TabsTrigger>
}
