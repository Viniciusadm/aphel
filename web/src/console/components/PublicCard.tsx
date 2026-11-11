import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Card, CardContent, CardDescription, CardHeader } from '@/components/ui/card'
import { PageTitle } from '@/components/ui/page-title'

interface PublicCardProps {
  title: string
  description?: ReactNode
  children?: ReactNode
}

export function PublicCard({ title, description, children }: PublicCardProps) {
  const { t } = useTranslation('console')
  return (
    <div className="flex min-h-dvh flex-col items-center justify-center gap-6 bg-background px-4 py-10">
      <p className="text-sm font-semibold text-muted-foreground">{t('brand')}</p>
      <Card className="w-full max-w-sm">
        <CardHeader>
          <PageTitle>{title}</PageTitle>
          {description && <CardDescription>{description}</CardDescription>}
        </CardHeader>
        {children && <CardContent>{children}</CardContent>}
      </Card>
    </div>
  )
}
