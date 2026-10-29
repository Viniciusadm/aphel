import type { RecordItem } from '@/features/records/types'
import { activeContributions, sortContributions } from './registry'

export interface ModuleEventMap {
  'record.created': { recordType: string; record: RecordItem }
}

export type ModuleEventName = keyof ModuleEventMap

export type ModuleEventHandler<N extends ModuleEventName> = (payload: ModuleEventMap[N]) => void

export function emitModuleEvent<N extends ModuleEventName>(name: N, payload: ModuleEventMap[N]) {
  for (const { value } of sortContributions(activeContributions<ModuleEventHandler<N>>(`on:${name}`))) {
    value(payload)
  }
}
