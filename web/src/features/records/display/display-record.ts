import { createContext, useContext } from 'react'

export interface DisplayRecord {
  recordType: string
  recordId: string
  title?: string | null
}

const DisplayRecordContext = createContext<DisplayRecord | null>(null)

export const DisplayRecordProvider = DisplayRecordContext.Provider

export function useDisplayRecord(): DisplayRecord | null {
  return useContext(DisplayRecordContext)
}
