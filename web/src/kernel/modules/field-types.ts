import { useMemo, type ComponentType } from 'react'
import type { FieldInputProps } from '@/components/fields/FieldInput'
import type { FieldDefinition } from '@/features/records/types'
import { useActiveContributions } from './registry'

export interface FieldDisplayProps {
  field: FieldDefinition
  value: unknown
}

export interface FieldTypeRenderers {
  Input?: ComponentType<FieldInputProps>
  Display?: ComponentType<FieldDisplayProps>
}

const nativeFieldTypes = new Map<string, FieldTypeRenderers>()

export function registerNativeFieldTypes(renderers: Record<string, FieldTypeRenderers>) {
  for (const [type, value] of Object.entries(renderers)) {
    nativeFieldTypes.set(type, { ...nativeFieldTypes.get(type), ...value })
  }
}

export function useFieldTypeRenderers(type: string, fieldKey?: string): FieldTypeRenderers {
  const contributed = useActiveContributions<FieldTypeRenderers>(`fieldType:${type}`)
  const specific = useActiveContributions<FieldTypeRenderers>(`fieldType:${type}:${fieldKey ?? ''}`)
  return useMemo(() => {
    const native = nativeFieldTypes.get(type) ?? {}
    const pick = <K extends keyof FieldTypeRenderers>(part: K): FieldTypeRenderers[K] =>
      specific.findLast((item) => item.value[part])?.value[part] ??
      contributed.findLast((item) => item.value[part])?.value[part] ??
      native[part]
    return { Input: pick('Input'), Display: pick('Display') }
  }, [contributed, specific, type])
}
