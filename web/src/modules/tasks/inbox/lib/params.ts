import type {InboxOwner, InboxQuery, InboxSort} from '@/sdk'

export const INBOX_OWNERS: InboxOwner[] = ['me', 'team', 'all']
export const INBOX_SORTS: InboxSort[] = ['due', 'priority', 'value']

export const INBOX_PARAMS = { owner: 'owner', team: 'team', sort: 'sort' } as const

export function parseInboxQuery(params: URLSearchParams): InboxQuery {
  const owner = params.get(INBOX_PARAMS.owner)
  const sort = params.get(INBOX_PARAMS.sort)
  const teamId = params.get(INBOX_PARAMS.team) || undefined
  return {
    owner: INBOX_OWNERS.includes(owner as InboxOwner) ? (owner as InboxOwner) : 'me',
    sort: INBOX_SORTS.includes(sort as InboxSort) ? (sort as InboxSort) : 'due',
    teamId: owner === 'team' ? teamId : undefined,
  }
}

export function buildInboxParams(query: InboxQuery, base: URLSearchParams): URLSearchParams {
  const next = new URLSearchParams(base)
  if (query.owner === 'me') next.delete(INBOX_PARAMS.owner)
  else next.set(INBOX_PARAMS.owner, query.owner)
  if (query.sort === 'due') next.delete(INBOX_PARAMS.sort)
  else next.set(INBOX_PARAMS.sort, query.sort)
  if (query.owner === 'team' && query.teamId) next.set(INBOX_PARAMS.team, query.teamId)
  else next.delete(INBOX_PARAMS.team)
  return next
}
