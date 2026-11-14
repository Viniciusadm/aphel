import { ArrowRight, type LucideIcon } from 'lucide-react'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'

export type ConnectionState = 'on' | 'off' | 'attention'

const TONES = { on: 'success', off: 'neutral', attention: 'warning' } as const

export interface ConnectionCardProps {
  icon: LucideIcon
  title: string
  description: string
  state: ConnectionState
  stateLabel: string
  detail?: string
  to: string
  actionLabel: string
}

export function ConnectionCard({ icon: Icon, title, description, state, stateLabel, detail, to, actionLabel }: ConnectionCardProps) {
  return (
    <li className="contents">
      <Card className="gap-4 py-4">
        <CardHeader className="px-4">
          <div className="flex items-start justify-between gap-3">
            <div className="flex min-w-0 items-center gap-3">
              <span className="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted">
                <Icon className="size-5" aria-hidden="true" />
              </span>
              <CardTitle className="break-words">{title}</CardTitle>
            </div>
            <Badge tone={TONES[state]}>{stateLabel}</Badge>
          </div>
          <CardDescription>{description}</CardDescription>
        </CardHeader>
        {detail && <CardContent className="px-4 text-sm text-muted-foreground">{detail}</CardContent>}
        <CardFooter className="mt-auto px-4">
          <Button asChild size="sm" variant="outline">
            <Link to={to}>
              {actionLabel}
              <ArrowRight aria-hidden="true" />
            </Link>
          </Button>
        </CardFooter>
      </Card>
    </li>
  )
}
