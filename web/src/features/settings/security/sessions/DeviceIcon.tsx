import { Bot, CircleHelp, Monitor, Smartphone, Tablet, type LucideIcon } from 'lucide-react'
import type { SecurityDevice } from './api'

const DEVICE_ICONS: Record<SecurityDevice['kind'], LucideIcon> = {
  desktop: Monitor,
  mobile: Smartphone,
  tablet: Tablet,
  bot: Bot,
  unknown: CircleHelp,
}

export function DeviceIcon({ device, className }: { device: SecurityDevice; className?: string }) {
  const Icon = DEVICE_ICONS[device.kind]
  return <Icon aria-hidden="true" className={className} />
}
