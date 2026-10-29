import {
  Briefcase,
  Calendar,
  CircleDot,
  Coffee,
  FileText,
  Flag,
  Handshake,
  Mail,
  MapPin,
  MessageCircle,
  Phone,
  PhoneForwarded,
  Presentation,
  Repeat,
  Send,
  SquareCheck,
  Star,
  Users,
  Video,
  type LucideIcon,
} from 'lucide-react'

export const ACTIVITY_ICONS: Record<string, LucideIcon> = {
  phone: Phone,
  'phone-forwarded': PhoneForwarded,
  users: Users,
  mail: Mail,
  send: Send,
  'message-circle': MessageCircle,
  'map-pin': MapPin,
  'check-square': SquareCheck,
  calendar: Calendar,
  video: Video,
  presentation: Presentation,
  handshake: Handshake,
  briefcase: Briefcase,
  coffee: Coffee,
  'file-text': FileText,
  repeat: Repeat,
  flag: Flag,
  star: Star,
}

export const ACTIVITY_ICON_NAMES = Object.keys(ACTIVITY_ICONS)

export const DEFAULT_ACTIVITY_COLOR = '#6e7781'

export function activityIcon(name: string | null | undefined): LucideIcon {
  return (name && ACTIVITY_ICONS[name]) || CircleDot
}
