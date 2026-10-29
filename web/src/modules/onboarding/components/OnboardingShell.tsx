import type { ReactNode } from 'react'
import { Brand } from '@/app/layout/Brand'

export function OnboardingShell({ children }: { children: ReactNode }) {
  return (
    <div className="flex min-h-screen flex-col bg-muted/40 text-foreground">
      <header className="flex h-14 items-center px-4">
        <Brand />
      </header>
      <main className="flex flex-1 items-start justify-center px-4 py-6 sm:py-10">
        <div className="w-full max-w-2xl">{children}</div>
      </main>
    </div>
  )
}
