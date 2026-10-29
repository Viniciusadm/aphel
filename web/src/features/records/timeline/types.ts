export interface TimelineActor {
  id: string
  display_name: string
}

export interface TimelineRecordReference {
  record_type: string
  record_id: string
  title?: string | null
}

export interface TimelineItem {
  id: string
  kind: string
  occurred_at: string
  actor: TimelineActor | null
  title: string
  body?: string | null
  record?: TimelineRecordReference | null
  metadata: Record<string, unknown>
}

export interface TimelinePageMeta {
  next_cursor?: string | null
}

export interface TimelinePage {
  data: TimelineItem[]
  meta?: TimelinePageMeta
}

export interface TimelinePageRequest {
  cursor: string | null
  kinds: string[]
}

export type TimelineFetchPage = (request: TimelinePageRequest) => Promise<TimelinePage>

export interface TimelineKindOption {
  kind: string
  label: string
}

export interface TimelineSource {
  fetchPage: TimelineFetchPage
  kindOptions?: TimelineKindOption[]
}
