import { Outlet } from 'react-router'
import { Brand } from '@/app/layout/Brand'
import { PublicModulesSync } from '@/kernel/modules/ModulesSync'
import { Slot } from '@/kernel/modules/Slot'

export function AuthLayout() {
  return (
    <div className="flex min-h-screen flex-col bg-muted/40 text-foreground">
      <PublicModulesSync />
      <Slot name="app.banner" props={{}} />
      <header className="flex h-14 items-center px-4">
        <Brand />
      </header>
      <main className="flex flex-1 items-start justify-center px-4 py-8 sm:items-center">
        <div className="w-full max-w-sm">
          <Outlet />
        </div>
      </main>
    </div>
  )
}
