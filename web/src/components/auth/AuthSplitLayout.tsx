import type { ReactNode } from 'react'

interface AuthSplitLayoutProps {
  brand: ReactNode
  tagline: string
  subline?: string
  top?: ReactNode
  children: ReactNode
}

function OrbitArt({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 400 400" fill="none" aria-hidden className={className}>
      <g stroke="currentColor" strokeWidth="1.5">
        <ellipse cx="200" cy="200" rx="190" ry="80" transform="rotate(-28 200 200)" opacity="0.35" />
        <ellipse cx="200" cy="200" rx="150" ry="62" transform="rotate(-28 200 200)" opacity="0.25" />
        <ellipse cx="200" cy="200" rx="110" ry="110" opacity="0.18" />
        <ellipse cx="200" cy="200" rx="70" ry="70" opacity="0.14" />
      </g>
      <circle cx="200" cy="200" r="16" fill="currentColor" opacity="0.9" />
      <circle cx="368" cy="111" r="7" fill="currentColor" opacity="0.8" />
      <circle cx="84" cy="273" r="4.5" fill="currentColor" opacity="0.6" />
    </svg>
  )
}

export function AuthSplitLayout({ brand, tagline, subline, top, children }: AuthSplitLayoutProps) {
  return (
    <div className="flex min-h-dvh flex-col bg-background text-foreground">
      {top}
      <div className="grid flex-1 lg:grid-cols-[minmax(0,5fr)_minmax(0,6fr)]">
        <aside className="relative isolate overflow-hidden bg-primary-solid text-primary-foreground">
          <div
            aria-hidden
            className="absolute inset-0 -z-10 bg-[radial-gradient(circle_at_20%_10%,color-mix(in_oklab,white_22%,transparent),transparent_55%),radial-gradient(circle_at_90%_100%,color-mix(in_oklab,black_28%,transparent),transparent_60%)]"
          />
          <OrbitArt className="pointer-events-none absolute -top-24 -right-32 hidden size-[36rem] lg:block" />
          <OrbitArt className="pointer-events-none absolute -top-16 -right-14 size-48 opacity-60 lg:hidden" />
          <div className="relative flex h-full flex-col gap-10 px-4 py-4 sm:px-6 lg:px-12 lg:py-10">
            {brand}
            <div className="hidden max-w-md flex-1 flex-col justify-end gap-4 lg:flex">
              <p className="text-4xl leading-tight font-semibold tracking-tight text-balance [font-family:var(--font-family-heading)]">
                {tagline}
              </p>
              {subline && <p className="text-base leading-relaxed text-pretty opacity-85">{subline}</p>}
            </div>
          </div>
        </aside>
        <main className="flex items-start justify-center px-4 py-10 sm:px-6 sm:py-14 lg:items-center lg:px-12">
          <div className="w-full max-w-sm">{children}</div>
        </main>
      </div>
    </div>
  )
}
