import { createContext, useContext } from 'react'

export interface SetupFrameContextValue {
  onBack?: () => void
}

export const SetupFrameContext = createContext<SetupFrameContextValue>({})

export function useSetupFrame(): SetupFrameContextValue {
  return useContext(SetupFrameContext)
}
