import type { LucideIcon } from 'lucide-react'
import { Link } from 'react-router'

interface SettingsLinkCardProps {
  to: string
  icon: LucideIcon
  title: string
  description: string
}

export function SettingsLinkCard({ to, icon: Icon, title, description }: SettingsLinkCardProps) {
  return (
    <Link
      to={to}
      className="flex items-start gap-3 rounded-xl border bg-card p-4 text-card-foreground shadow-xs transition-colors hover:bg-accent/50 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
    >
      <span className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
        <Icon aria-hidden className="size-4" />
      </span>
      <span className="grid gap-0.5">
        <span className="text-sm font-medium">{title}</span>
        <span className="text-sm text-muted-foreground">{description}</span>
      </span>
    </Link>
  )
}
