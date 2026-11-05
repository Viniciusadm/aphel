import type { components } from '@/modules/followers/api-types'

type Schemas = components['schemas']

export type FollowOrigin = 'manual' | 'owner'

export type RecordFollower = Omit<Schemas['RecordFollowerResponse'], 'origin'> & { origin: FollowOrigin }
export type RecordFollowers = Omit<Schemas['RecordFollowersResponse'], 'followers'> & { followers: RecordFollower[] }
export type FollowedRecord = Omit<Schemas['FollowedRecordResponse'], 'origin'> & { origin: FollowOrigin }
export type FollowedRecordsPage = Omit<Schemas['Page_FollowedRecordResponse'], 'data'> & { data: FollowedRecord[] }
