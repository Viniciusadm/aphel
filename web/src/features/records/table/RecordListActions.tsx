import { useTranslation } from 'react-i18next'
import { WorkspaceOptions } from '@/components/ui/workspace-options'
import { useSlot } from '@/kernel/modules/slots'

export function RecordListActions({ recordType }: { recordType: string }) {
  const { t } = useTranslation('records-table')
  const actions = useSlot('record.list.actions', { recordType })
  if (!actions.length) return null
  return <WorkspaceOptions label={t('collectionActions')}>
    {actions.map(({ key, Component }) => <Component key={key} recordType={recordType} />)}
  </WorkspaceOptions>
}
