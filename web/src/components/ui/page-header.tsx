import { ArrowLeft } from 'lucide-react'
import type { ReactNode } from 'react'
import { Link } from 'react-router'
import { cn } from '@/lib/utils'
import { useUiLabels } from './labels-context'

export interface PageHeaderBack {
  to: string
  label?: string
}

export interface PageHeaderProps {
  title: string
  description?: ReactNode
  actions?: ReactNode
  back?: PageHeaderBack
  badges?: ReactNode
  level?: 'page' | 'section'
  className?: string
}

export function PageHeader({ title, description, actions, back, badges, level = 'page', className }: PageHeaderProps) {
  const labels = useUiLabels()
  const Heading = level === 'page' ? 'h1' : 'h2'
  return (
    <header data-slot="page-header" className={cn('flex flex-col gap-2', className)}>
      {back && (
        <Link
          to={back.to}
          className="inline-flex w-fit items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft className="size-4" aria-hidden />
          {back.label ?? labels.back}
        </Link>
      )}
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="grid min-w-0 gap-1">
          <div className="flex flex-wrap items-center gap-2">
            <Heading
              className={cn(
                'font-semibold tracking-tight [font-family:var(--font-family-heading)]',
                level === 'page' ? 'text-xl' : 'text-lg',
              )}
            >
              {title}
            </Heading>
            {badges}
          </div>
          {description && <div className="grid gap-1 text-sm text-muted-foreground">{description}</div>}
        </div>
        {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
      </div>
    </header>
  )
}
