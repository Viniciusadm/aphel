import { FileUp } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import type { SlotProps } from '@/kernel/modules/slots'
import { importPermission } from './api'

export function ImportAction({ recordType }: SlotProps<'record.list.toolbar'>) {
  const { t } = useTranslation('imports')
  const { hasPermission } = useSession()
  const permission = importPermission(recordType)
  if (permission === null || !hasPermission(permission)) return null
  return (
    <Button asChild size="sm" variant="outline">
      <Link to={`/imports/new?record_type=${encodeURIComponent(recordType)}`}>
        <FileUp aria-hidden />
        {t('actions.import')}
      </Link>
    </Button>
  )
}
