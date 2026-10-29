import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  API_BASE_URL,
  ApiError,
  CSRF_HEADER,
  CSRF_HEADER_VALUE,
  NETWORK_ERROR_CODE,
  UNKNOWN_ERROR_CODE,
  api,
  parseErrorEnvelope,
  type components,
} from '@/api'

export type Attachment = components['schemas']['AttachmentResponse']
export type StoredFile = components['schemas']['FileResponse']
export type UploadedFile = components['schemas']['UploadFileResponse']
export type AttachmentsPage = components['schemas']['Page_AttachmentResponse']

export const ATTACHMENTS_PER_PAGE = 100

export interface RecordTarget {
  recordType: string
  recordId: string
}

export interface UploadFileInput {
  file: File
  target?: RecordTarget
  onProgress?: (fraction: number) => void
  signal?: AbortSignal
}

export const fileKeys = {
  all: ['files'] as const,
  attachments: ({ recordType, recordId }: RecordTarget) =>
    [...fileKeys.all, 'attachments', recordType, recordId] as const,
}

export function fileDownloadUrl(fileId: string): string {
  return `${API_BASE_URL}/v1/files/${encodeURIComponent(fileId)}`
}

function parseJson(text: string): unknown {
  if (!text) return undefined
  try {
    return JSON.parse(text)
  } catch {
    return text
  }
}

function unwrapData(payload: unknown): unknown {
  if (typeof payload !== 'object' || payload === null || Array.isArray(payload)) return payload
  const keys = Object.keys(payload)
  return keys.length === 1 && keys[0] === 'data' ? (payload as { data: unknown }).data : payload
}

export function uploadFile({ file, target, onProgress, signal }: UploadFileInput): Promise<UploadedFile> {
  return new Promise((resolve, reject) => {
    const request = new XMLHttpRequest()
    request.open('POST', `${API_BASE_URL}/v1/files`)
    request.withCredentials = true
    request.setRequestHeader('Accept', 'application/json')
    request.setRequestHeader(CSRF_HEADER, CSRF_HEADER_VALUE)

    request.upload.onprogress = (event) => {
      if (event.lengthComputable && event.total > 0) onProgress?.(event.loaded / event.total)
    }
    request.onload = () => {
      const payload = parseJson(request.responseText)
      if (request.status >= 200 && request.status < 300) {
        onProgress?.(1)
        resolve(unwrapData(payload) as UploadedFile)
        return
      }
      reject(
        new ApiError(
          request.status,
          parseErrorEnvelope(payload) ?? {
            code: UNKNOWN_ERROR_CODE,
            message: request.statusText || `HTTP ${request.status}`,
          },
        ),
      )
    }
    request.onerror = () => reject(new ApiError(0, { code: NETWORK_ERROR_CODE, message: 'network error' }))
    request.onabort = () => reject(new DOMException('upload aborted', 'AbortError'))
    signal?.addEventListener('abort', () => request.abort(), { once: true })

    const form = new FormData()
    if (target) {
      form.append('record_type', target.recordType)
      form.append('record_id', target.recordId)
    }
    form.append('file', file, file.name)
    request.send(form)
  })
}

export const filesApi = {
  listAttachments: ({ recordType, recordId }: RecordTarget, page = 1, perPage = ATTACHMENTS_PER_PAGE) =>
    api.get<AttachmentsPage>('/v1/attachments', {
      query: { record_type: recordType, record_id: recordId, page, per_page: perPage },
    }),
  removeAttachment: (attachmentId: string) =>
    api.delete<void>(`/v1/attachments/${encodeURIComponent(attachmentId)}`),
  uploadFile,
}

export function useAttachments(target: RecordTarget) {
  return useQuery({
    queryKey: fileKeys.attachments(target),
    queryFn: () => filesApi.listAttachments(target),
  })
}

export function useUploadAttachment(target: RecordTarget) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: Omit<UploadFileInput, 'target'>) => filesApi.uploadFile({ ...input, target }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: fileKeys.attachments(target) }),
  })
}

export function useRemoveAttachment(target: RecordTarget) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (attachmentId: string) => filesApi.removeAttachment(attachmentId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: fileKeys.attachments(target) }),
  })
}
