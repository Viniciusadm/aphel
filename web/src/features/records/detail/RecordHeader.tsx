import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { PageHeader } from '@/components/ui/page-header'

export interface RecordBackLink {
  to: string
  label?: string
}

export interface RecordHeaderProps {
  title: ReactNode
  titleAccessory?: ReactNode
  subtitle?: ReactNode
  owner?: ReactNode
  actions?: ReactNode
  back?: RecordBackLink
  className?: string
}

export function RecordHeader({ title, titleAccessory, subtitle, owner, actions, back, className }: RecordHeaderProps) {
  const { t } = useTranslation('records-detail')
  return (
    <PageHeader
      className={className}
      title={title}
      badges={titleAccessory}
      back={back && { ...back, label: back.label ?? t('header.back') }}
      actions={actions}
      description={(subtitle || owner) && (
        <div className="flex min-w-0 flex-wrap items-center gap-x-4 gap-y-1">
          {subtitle && <div className="min-w-0 max-w-full truncate">{subtitle}</div>}
          {owner && <div className="flex min-w-0 items-center gap-2">
            <span>{t('header.owner')}:</span>
            <span className="min-w-0 text-foreground">{owner}</span>
          </div>}
        </div>
      )}
    />
  )
}
