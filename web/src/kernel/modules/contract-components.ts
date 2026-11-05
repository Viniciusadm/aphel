import { useMemo, type ComponentType } from 'react'
import { sortContributions, useActiveContributions } from './registry'

export interface ContractComponentMap {}

export type ContractComponentKey = keyof ContractComponentMap

export type ContractComponentProps<K extends ContractComponentKey> = ContractComponentMap[K]

const NothingProvided = () => null

export function contractBucket(key: string): string {
  return `contractComponent:${key}`
}

export function useContractComponent<K extends ContractComponentKey>(
  key: K,
  Fallback: ComponentType<ContractComponentProps<K>> | null = null,
): ComponentType<ContractComponentProps<K>> {
  const providers = useActiveContributions<ComponentType<ContractComponentProps<K>>>(contractBucket(key))
  return useMemo(() => {
    const [first] = sortContributions(providers)
    return first?.value ?? Fallback ?? NothingProvided
  }, [providers, Fallback])
}

export function useHasContractComponent(key: ContractComponentKey): boolean {
  return useActiveContributions<unknown>(contractBucket(key)).length > 0
}

export function useContractProviders<K extends ContractComponentKey>(key: K): ComponentType<ContractComponentProps<K>>[] {
  const providers = useActiveContributions<ComponentType<ContractComponentProps<K>>>(contractBucket(key))
  return useMemo(() => sortContributions(providers).map(({ value }) => value), [providers])
}
