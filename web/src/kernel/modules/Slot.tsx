import { useSlot, type SlotName, type SlotProps } from './slots'

export interface SlotRenderProps<N extends SlotName> {
  name: N
  props: SlotProps<N>
}

export function Slot<N extends SlotName>({ name, props }: SlotRenderProps<N>) {
  const entries = useSlot(name, props)
  return entries.map(({ key, Component }) => <Component key={key} {...props} />)
}
