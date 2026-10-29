export interface QuoteEditorSlotProps {
  recordType: string
  recordId: string
}

declare module '@/kernel/modules/slots' {
  interface SlotPropsMap {
    'quotes.editor.sidebar': QuoteEditorSlotProps
  }
}
