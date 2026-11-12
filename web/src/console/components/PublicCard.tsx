import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Brand } from '@/app/layout/Brand'
import { AuthPanel } from '@/components/auth/AuthPanel'
import { AuthSplitLayout } from '@/components/auth/AuthSplitLayout'

interface PublicCardProps {
  title: string
  description?: ReactNode
  children?: ReactNode
}

export function PublicCard({ title, description, children }: PublicCardProps) {
  const { t } = useTranslation('console')
  return (
    <AuthSplitLayout brand={<Brand tone="inverted" label={t('brand')} />} tagline={t('layout.tagline')} subline={t('layout.subline')}>
      <AuthPanel title={title} description={description}>
        {children}
      </AuthPanel>
    </AuthSplitLayout>
  )
}
