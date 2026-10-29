import { LinkIcon } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { EmptyState } from '@/components/ui/empty-state'
import { BookingLayout } from './BookingLayout'

export function InvalidLink() {
  const { t } = useTranslation('scheduling', { keyPrefix: 'public' })
  return (
    <BookingLayout>
      <div className="p-6">
        <EmptyState
          icon={LinkIcon}
          title={t('invalid.title')}
          description={t('invalid.description')}
          className="border-0"
        />
      </div>
    </BookingLayout>
  )
}
