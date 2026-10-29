import type { components, operations } from '@/api'

type Schemas = components['schemas']

export type CheckStatus = 'ok' | 'fail' | 'skipped'
export type FailureSource = 'automation' | 'webhook' | 'integration'

const CHECK_STATUSES: readonly CheckStatus[] = ['ok', 'fail', 'skipped']

export function asCheckStatus(value: string): CheckStatus {
  return CHECK_STATUSES.find((status) => status === value) ?? 'fail'
}

export type HealthCheck = Schemas['HealthCheckResponse']
export type Readiness = Schemas['ReadinessResponse']
export type Heartbeat = Schemas['HeartbeatResponse']
export type InstalledModule = Schemas['InstalledModuleResponse']
export type SystemOverview = Schemas['SystemOverviewResponse']
export type QueueStats = Schemas['QueueStatsResponse']
export type JobKindStats = Schemas['JobKindStatsResponse']
export type JobStats = Schemas['JobStatsResponse']
export type FailedJob = Schemas['FailedJobResponse']
export type FailedJobsPage = Schemas['Page_FailedJobResponse']
export type FailedJobsQuery = NonNullable<operations['system_failed_jobs']['parameters']['query']>
export type Failure = Schemas['FailureResponse']
export type RecentFailures = Schemas['RecentFailuresResponse']
export type MigrationEntry = Schemas['MigrationEntryResponse']
export type CoreMigrations = Schemas['CoreMigrationsResponse']
export type ModuleMigrations = Schemas['ModuleMigrationsResponse']
export type Migrations = Schemas['MigrationsResponse']
