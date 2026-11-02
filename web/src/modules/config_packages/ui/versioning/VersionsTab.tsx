import { useTranslation } from 'react-i18next'
import { SectionLink } from '@/components/ui/section-workspace'
import { VERSIONS_TAB } from '@/modules/config_packages/ui/versioning/tab'

export function VersionsTabTrigger() {
  const { t } = useTranslation('config-package-versioning')
  return <SectionLink value={VERSIONS_TAB}>{t('tab')}</SectionLink>
}
