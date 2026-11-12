import type { ReactNode } from 'react'

interface AuthPanelProps {
  title: string
  description?: ReactNode
  footer?: ReactNode
  children?: ReactNode
}

export function AuthPanel({ title, description, footer, children }: AuthPanelProps) {
  return (
    <section className="grid gap-8">
      <header className="grid gap-2">
        <h1 className="text-2xl font-semibold tracking-tight text-balance [font-family:var(--font-family-heading)]">{title}</h1>
        {description && <div className="text-sm leading-relaxed text-pretty text-muted-foreground">{description}</div>}
      </header>
      {children && <div>{children}</div>}
      {footer && (
        <footer className="flex flex-wrap items-center justify-center gap-x-1 border-t pt-6 text-sm text-muted-foreground">{footer}</footer>
      )}
    </section>
  )
}
