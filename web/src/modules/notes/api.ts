import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'
import { recordsKeys } from '@/features/records/api'


export type Note = components['schemas']['NoteResponse']
export type NotePage = components['schemas']['Page_NoteResponse']
export type NoteVisibility = components['schemas']['NoteVisibility']
export type CreateNoteRequest = components['schemas']['CreateNoteRequest']
export type UpdateNoteRequest = components['schemas']['UpdateNoteRequest']

export const NOTES_PAGE_SIZE = 100

export interface NoteTarget {
  recordType: string
  recordId: string
}

export const notesKeys = {
  all: ['notes'] as const,
  forRecord: (target: NoteTarget) => [...notesKeys.all, target.recordType, target.recordId] as const,
}

function notePath(id: string, suffix = '') {
  return `/v1/notes/${encodeURIComponent(id)}${suffix}`
}

export const notesApi = {
  list: (target: NoteTarget) =>
    api.get<NotePage>('/v1/notes', {
      query: { record_type: target.recordType, record_id: target.recordId, per_page: NOTES_PAGE_SIZE },
    }),
  create: (request: CreateNoteRequest) => api.post<Note>('/v1/notes', request),
  update: (id: string, request: UpdateNoteRequest) => api.patch<Note>(notePath(id), request),
  setPinned: (id: string, pinned: boolean) => api.post<Note>(notePath(id, pinned ? '/pin' : '/unpin')),
  remove: (id: string) => api.delete<void>(notePath(id)),
}

export function useRecordNotes(target: NoteTarget) {
  return useQuery({ queryKey: notesKeys.forRecord(target), queryFn: () => notesApi.list(target) })
}

function useInvalidateNotes(target: NoteTarget) {
  const queryClient = useQueryClient()
  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: notesKeys.forRecord(target) }),
      queryClient.invalidateQueries({ queryKey: recordsKeys.detail(target.recordType, target.recordId) }),
    ])
}

export function useCreateNote(target: NoteTarget) {
  const invalidate = useInvalidateNotes(target)
  return useMutation({
    mutationFn: (request: Omit<CreateNoteRequest, 'record_type' | 'record_id'>) =>
      notesApi.create({ ...request, record_type: target.recordType, record_id: target.recordId }),
    onSuccess: () => invalidate(),
  })
}

export function useUpdateNote(target: NoteTarget) {
  const invalidate = useInvalidateNotes(target)
  return useMutation({
    mutationFn: ({ id, request }: { id: string; request: UpdateNoteRequest }) => notesApi.update(id, request),
    onSuccess: () => invalidate(),
  })
}

export function usePinNote(target: NoteTarget) {
  const invalidate = useInvalidateNotes(target)
  return useMutation({
    mutationFn: ({ id, pinned }: { id: string; pinned: boolean }) => notesApi.setPinned(id, pinned),
    onSuccess: () => invalidate(),
  })
}

export function useDeleteNote(target: NoteTarget) {
  const invalidate = useInvalidateNotes(target)
  return useMutation({ mutationFn: notesApi.remove, onSuccess: () => invalidate() })
}
