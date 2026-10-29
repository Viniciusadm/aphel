import {
  Box,
  Briefcase,
  Building2,
  CalendarDays,
  Car,
  ClipboardList,
  FileText,
  Folder,
  GraduationCap,
  Handshake,
  Heart,
  House,
  KeyRound,
  Layers,
  Lightbulb,
  MapPin,
  Package,
  Receipt,
  Shield,
  Star,
  Tag,
  Ticket,
  Truck,
  Wrench,
  type LucideIcon,
} from 'lucide-react'

export const OBJECT_ICONS: Record<string, LucideIcon> = {
  box: Box,
  'file-text': FileText,
  folder: Folder,
  briefcase: Briefcase,
  building: Building2,
  house: House,
  car: Car,
  package: Package,
  'clipboard-list': ClipboardList,
  calendar: CalendarDays,
  star: Star,
  tag: Tag,
  heart: Heart,
  shield: Shield,
  wrench: Wrench,
  truck: Truck,
  'graduation-cap': GraduationCap,
  ticket: Ticket,
  receipt: Receipt,
  key: KeyRound,
  'map-pin': MapPin,
  layers: Layers,
  handshake: Handshake,
  lightbulb: Lightbulb,
}

export const OBJECT_ICON_NAMES = Object.keys(OBJECT_ICONS)

export const DEFAULT_OBJECT_ICON = 'box'

export const OBJECT_COLORS = [
  '#2563eb',
  '#7c3aed',
  '#db2777',
  '#dc2626',
  '#ea580c',
  '#ca8a04',
  '#16a34a',
  '#0d9488',
  '#0891b2',
  '#475569',
]

export const DEFAULT_OBJECT_COLOR = OBJECT_COLORS[0] ?? '#2563eb'

export const HEX_COLOR_PATTERN = /^#[0-9a-f]{6}$/i

export function objectIcon(name: string | null | undefined): LucideIcon {
  return (name && OBJECT_ICONS[name]) || Box
}

export function objectColor(color: string | null | undefined): string | undefined {
  return color && HEX_COLOR_PATTERN.test(color) ? color : undefined
}
