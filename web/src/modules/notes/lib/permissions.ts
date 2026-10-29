import { useSession } from '@/app/session/session-context'
import type { Note } from '../api'

export function useNotePermissions(note: Note) {
  const { me, hasPermission, permissionScope } = useSession()
  const isAuthor = note.author_id === me.user.id
  const allowed = (permission: string) => {
    if (isAuthor) return hasPermission(permission)
    if (note.visibility === 'private') return false
    const scope = permissionScope(permission)
    return scope === 'organization' || scope === 'team'
  }
  return { isAuthor, canEdit: allowed('notes.update'), canDelete: allowed('notes.delete') }
}
