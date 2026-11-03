declare module '@/kernel/modules/slots' {
  interface SlotPropsMap {
    'line_items.catalogPicker': { currency: string; disabled: boolean; onSelect: (id: string) => void }
  }
}
export {}
