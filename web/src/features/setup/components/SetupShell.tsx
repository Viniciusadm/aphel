import type { ReactNode } from 'react'
import { Brand } from '@/app/layout/Brand'

export function SetupShell({ progress, children }: { progress?: ReactNode; children: ReactNode }) {
  return (
    <div className="flex min-h-dvh flex-col bg-muted/40 text-foreground">
      <header className="flex h-14 items-center px-4">
        <Brand />
      </header>
      <main className="flex flex-1 justify-center px-4 pt-2 pb-10 sm:pt-6">
        <div className="grid w-full max-w-2xl content-start gap-6">
          {progress}
          {children}
        </div>
      </main>
    </div>
  )
}
