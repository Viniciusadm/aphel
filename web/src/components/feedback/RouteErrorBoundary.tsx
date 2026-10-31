import { PageTitle } from '@/components/ui/page-title'
import { useQueryClient } from '@tanstack/react-query'
import { CircleAlert } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link, isRouteErrorResponse, useLocation, useNavigate, useRouteError } from 'react-router'
import { isApiError } from '@/api'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

export interface RouteErrorBoundaryProps {
  homePath?: string
  className?: string
}

function isNotFound(error: unknown) {
  return (isRouteErrorResponse(error) && error.status === 404) || (isApiError(error) && error.status === 404)
}

export function RouteErrorBoundary({ homePath = '/', className }: RouteErrorBoundaryProps) {
  const { t } = useTranslation('records-detail')
  const error = useRouteError()
  const location = useLocation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const notFound = isNotFound(error)

  const retry = () => {
    void queryClient.resetQueries({ predicate: (query) => query.state.status === 'error' })
    void navigate(`${location.pathname}${location.search}${location.hash}`, { replace: true })
  }

  return (
    <div
      role="alert"
      className={cn('mx-auto flex max-w-md flex-col items-center gap-4 px-4 py-16 text-center', className)}
    >
      <div className="flex size-12 items-center justify-center rounded-full bg-destructive/10 text-destructive">
        <CircleAlert className="size-6" aria-hidden="true" />
      </div>
      <div className="grid gap-1">
        <PageTitle >
          {notFound ? t('errorBoundary.notFoundTitle') : t('errorBoundary.title')}
        </PageTitle>
        <p className="text-sm text-muted-foreground">
          {notFound ? t('errorBoundary.notFoundDescription') : t('errorBoundary.routeDescription')}
        </p>
      </div>
      <div className="flex flex-wrap justify-center gap-2">
        {!notFound && <Button onClick={retry}>{t('errorBoundary.retry')}</Button>}
        <Button variant="outline" asChild>
          <Link to={homePath}>{t('errorBoundary.home')}</Link>
        </Button>
      </div>
    </div>
  )
}
