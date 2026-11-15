export interface QuoteEditorSlotProps {
  recordType: string
  recordId: string
  canRequest?: boolean
}

declare module '@/kernel/modules/slots' {
  interface SlotPropsMap {
    'quotes.editor.sidebar': QuoteEditorSlotProps
  }
}
