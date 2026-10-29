import type { CreateAction } from './create-actions'
import type { CreateRequest } from './types'

export interface CreateRecordDialogsProps {
  actions: CreateAction[]
  request: CreateRequest | null
  onClose: () => void
}

export function CreateRecordDialogs({ actions, request, onClose }: CreateRecordDialogsProps) {
  return actions.map(({ key, render: Render }) => {
    const open = request?.kind === key
    return <Render key={key} open={open} target={open ? request?.target : undefined} onClose={onClose} />
  })
}
