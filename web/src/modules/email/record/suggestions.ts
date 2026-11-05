import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'
import { api } from '@/api'
import { getRecordResource, type RecordItem, recordPath, recordsKeys, recordTitle } from '@/sdk'
import { useRecordThreads } from '../api'
import { isValidAddress, normalizeAddress, threadCounterparts } from '../lib/format'
import type { EmailAddress, EmailRecordRef } from '../types'

function useRecordItem(recordType: string, recordId: string | null) {
  const resource = getRecordResource(recordType)
  return useQuery({
    queryKey: recordsKeys.detail(recordType, recordId ?? ''),
    queryFn: () => api.get<RecordItem>(recordPath(resource!, recordId!)),
    enabled: Boolean(resource && recordId),
    staleTime: 60_000,
    retry: false,
  })
}

function addressesOf(recordType: string, record: RecordItem | undefined): EmailAddress[] {
  if (!record) return []
  const resource = getRecordResource(recordType)
  const name = resource ? recordTitle(resource, record) : undefined
  const values = [...Object.values(record), ...Object.values(record.custom_fields ?? {})]
  return values
    .filter((value): value is string => typeof value === 'string' && isValidAddress(value))
    .map((address) => ({ address: address.trim(), name: name && name !== record.id ? name : undefined }))
}

export function useRecipientSuggestions(record: EmailRecordRef): EmailAddress[] {
  const current = useRecordItem(record.record_type, record.record_id)
  const contactId = record.record_type !== 'people.person' && typeof current.data?.contact_id === 'string' ? current.data.contact_id : null
  const contact = useRecordItem('contact', contactId)
  const threads = useRecordThreads(record, 1)

  return useMemo(() => {
    const fromThreads = (threads.data?.data ?? []).flatMap((thread) =>
      threadCounterparts(thread).map((participant) => ({ address: participant.address, name: participant.name })),
    )
    const all = [
      ...addressesOf(record.record_type, current.data),
      ...addressesOf('contact', contact.data),
      ...fromThreads,
    ]
    const seen = new Set<string>()
    return all.filter((entry) => {
      const key = normalizeAddress(entry.address)
      if (seen.has(key)) return false
      seen.add(key)
      return true
    })
  }, [record.record_type, current.data, contact.data, threads.data])
}
