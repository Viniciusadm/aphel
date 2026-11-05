import type { ComponentType } from 'react'
import {
  useContractProviders,
  type ContractComponentKey,
  type ContractComponentProps,
} from './contract-components'

export type ContractComponentRenderProps<K extends ContractComponentKey> = ContractComponentProps<K> & {
  name: K
  fallback?: ComponentType<ContractComponentProps<K>> | null
}

export function ContractComponent<K extends ContractComponentKey>({
  name,
  fallback: Fallback = null,
  ...props
}: ContractComponentRenderProps<K>) {
  const [Provided] = useContractProviders(name)
  const rendered = props as unknown as ContractComponentProps<K>
  if (Provided) return <Provided {...rendered} />
  return Fallback ? <Fallback {...rendered} /> : null
}
