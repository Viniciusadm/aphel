declare module '@/kernel/modules/slots' {
  interface SlotPropsMap {
    'dashboard.source': { collection: string; source: string; data: unknown }
    'dashboard.top': Record<string, never>
  }
}

export {}
