export interface CalendarCreateProps {
  open: boolean
  dueAt: string | null
  durationMinutes: number | null
  onOpenChange: (open: boolean) => void
}

declare module '@/kernel/modules/slots' {
  interface SlotPropsMap {
    'calendar.create': CalendarCreateProps
  }
}
