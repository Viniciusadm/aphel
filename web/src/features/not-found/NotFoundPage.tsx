import { PageTitle } from '@/components/ui/page-title'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'

export function NotFoundPage() {
  const { t } = useTranslation()

  return (
    <div className="mx-auto flex max-w-md flex-col items-center gap-4 py-16 text-center">
      <p className="text-5xl font-bold text-primary">{t('notFound.code')}</p>
      <PageTitle>{t('notFound.title')}</PageTitle>
      <p className="text-sm text-muted-foreground">{t('notFound.description')}</p>
      <Button asChild>
        <Link to="/">{t('notFound.backHome')}</Link>
      </Button>
    </div>
  )
}
