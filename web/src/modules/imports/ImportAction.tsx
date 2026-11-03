import { FileUp } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'
import type { SlotProps } from '@/kernel/modules/slots'
import { useImportEntities } from './api'

export function ImportAction({ recordType }: SlotProps<'record.list.actions'>) {
  const { t } = useTranslation('imports')
  const entities = useImportEntities()
  if (!entities.data?.some((entity) => entity.record_type === recordType)) return null
  return (
    <Button asChild size="sm" variant="outline">
      <Link to={`/imports/new?record_type=${encodeURIComponent(recordType)}`}>
        <FileUp aria-hidden />
        {t('actions.import')}
      </Link>
    </Button>
  )
}
