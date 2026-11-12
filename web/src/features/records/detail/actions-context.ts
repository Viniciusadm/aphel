import { createContext } from 'react'

export const RecordActionsContext = createContext<{ close: () => void }>({ close: () => undefined })
