import { ArrowUpRight, BarChart3 } from 'lucide-react'
import { Link } from 'react-router'

export function ReportCard({ to, title, description }: { to: string; title: string; description: string }) {
  return (
    <Link
      to={to}
      className="grid content-start gap-4 rounded-xl border bg-card p-5 transition-colors hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
    >
      <BarChart3 className="size-6 text-primary" aria-hidden />
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-base font-semibold">{title}</h2>
        <ArrowUpRight className="size-4" aria-hidden />
      </div>
      <p className="text-sm text-muted-foreground">{description}</p>
    </Link>
  )
}
