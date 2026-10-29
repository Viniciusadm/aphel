export type DealTone = 'default' | 'warning'

export type DealBadgePlacement = 'card' | 'list'

export interface DealBadgesSlotProps {
  deal: object
  placement: DealBadgePlacement
}

export interface DealFieldLocksSlotProps {
  dealId: string
  lockFields: (source: string, fields: string[]) => void
}

declare module '@/kernel/modules/slots' {
  interface SlotPropsMap {
    'deals.badges': DealBadgesSlotProps
    'deals.detail.fieldLocks': DealFieldLocksSlotProps
  }
}

declare module '@/kernel/modules/filters' {
  interface FilterMap {
    'deals.tone': { value: DealTone; context: { deal: object } }
  }
}
