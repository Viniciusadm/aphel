import { createElement, type ComponentType } from 'react'
import { useOverride } from './overrides'

export type OverridableProps<P extends object> = { id: string; component: ComponentType<P> } & P

export function Overridable<P extends object>({ id, component, ...props }: OverridableProps<P>) {
  const Component = useOverride(id, component)
  return createElement(Component, props as P)
}
