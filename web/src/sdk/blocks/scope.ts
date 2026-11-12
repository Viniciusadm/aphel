import { createContext, useContext } from 'react'

export type BlockScope = 'content' | 'template'

export const BlockScopeContext = createContext<BlockScope>('content')

export function useBlockScope(): BlockScope {
  return useContext(BlockScopeContext)
}
