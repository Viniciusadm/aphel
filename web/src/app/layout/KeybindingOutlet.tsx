import { Outlet } from 'react-router'
import { KeybindingProvider } from '@/kernel/keybindings'

export function KeybindingOutlet() {
  return (
    <KeybindingProvider>
      <Outlet />
    </KeybindingProvider>
  )
}
