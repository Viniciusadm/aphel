import { useModuleCreateActions, type ModuleCreateAction } from '@/kernel/modules/actions'
import { recordListPath } from '@/features/records/links'

export type CreateAction = ModuleCreateAction

export function useCreateActions(): CreateAction[] {
  return useModuleCreateActions()
}

export function contextualCreateAction(pathname: string, actions: CreateAction[]): CreateAction | undefined {
  return actions.find((action) => {
    const listPath = action.recordType ? recordListPath(action.recordType) : undefined
    return listPath !== undefined && (pathname === listPath || pathname.startsWith(`${listPath}/`))
  })
}
