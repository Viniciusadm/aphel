export type Json = null | boolean | number | string | Json[] | { [key: string]: Json }
export type JsonObject = { [key: string]: Json }

export type ActorKind = 'user' | 'api_token' | 'automation' | 'plugin' | 'system' | 'anonymous'

export interface Actor {
  kind: ActorKind
  id: string | null
  userId: string | null
}

export * from './catalog'

export interface RecordRef {
  type: string
  id: string
}

export declare const hooks: {
  readonly SAVING: 'record.saving'
  readonly VALIDATING: 'record.validating'
  readonly SAVED: 'record.saved'
  readonly DELETING: 'record.deleting'
  readonly ARCHIVING: 'record.archiving'
  readonly COMMITTED: 'record.committed'
  readonly DELETED: 'record.deleted'
  readonly RESTORED: 'record.restored'
  readonly PURGED: 'record.purged'
}

export interface FieldError {
  field: string
  message: string
}

export declare class PluginError extends Error {
  constructor(code: string, message: string, options?: { status?: number; fields?: FieldError[] })
  readonly code: string
  readonly status: number | undefined
  readonly fields: FieldError[]
  static field(code: string, field: string, message: string): PluginError
  static notFound(code: string, message: string): PluginError
  static forbidden(code: string, message: string): PluginError
}

export interface Change {
  field: string
  old: Json
  new: Json
}

export interface HookCall {
  hook: string
  recordType: string
  recordId: string
  mode: 'create' | 'update' | null
  actor: Actor
  changes: Change[]
  values: JsonObject
  customFields: JsonObject
  readonly isCreate: boolean
  is(hook: string, recordType: string): boolean
  changed(field: string): Change | null
  customChanged(field: string): Change | null
}

export interface HookOutcome {
  values?: JsonObject
  customFields?: JsonObject
}

export interface PluginEvent {
  id: string
  type: string
  recordType: string | null
  recordId: string | null
  actor: Actor
  occurredAt: string
  payload: Json
}

export interface Request {
  method: string
  path: string
  route: string
  params: Record<string, string>
  query: Record<string, string>
  headers: [string, string][]
  body: Json
  actor: Actor
  is(method: string, route: string): boolean
  header(name: string): string | null
}

export interface Response {
  status: number
  body: Json | undefined
}

export interface ActionCall {
  name: string
  input: JsonObject
  recordType: string
  recordId: string
  record: JsonObject
  automationId: string
  runId: string
  actor: Actor
}

export interface WidgetRequest {
  name: string
  settings: JsonObject
  periodFrom: string
  periodTo: string
  timezone: string
  actor: Actor
}

export interface ListOptions {
  filter?: Json
  sort?: string
  search?: string
  page?: number
  perPage?: number
}

export interface Records {
  get(recordType: string, id: string): JsonObject | null
  list(recordType: string, options?: ListOptions): { data: JsonObject[]; total: number }
  create(recordType: string, values: JsonObject): JsonObject
  update(recordType: string, id: string, values: JsonObject): JsonObject
  delete(recordType: string, id: string): void
  updateIf(recordType: string, id: string, expectedUpdatedAt: string, values: JsonObject): JsonObject
  apply(mutations: RecordMutation[]): JsonObject[]
  aggregate(query: JsonObject): Json
  iterate(
    recordType: string,
    options?: ListOptions,
    cursor?: string | null,
  ): { data: JsonObject[]; cursor: string | null }
  ref(index: number): { $ref: number }
}

export interface RecordMutation {
  recordType: string
  id?: string | null
  expectedUpdatedAt?: string | null
  values: JsonObject
}

export interface Kv {
  get(id: string): Json | null
  set(id: string, value: Json): void
  delete(id: string): void
  list(): { id: string; value: Json }[]
  increment(id: string, delta?: number): number
  setIfAbsent(id: string, value: Json): boolean
}

export interface HttpRequest {
  clientIdentity?: string
  authentication?: string
  method?: string
  url: string
  headers?: Record<string, string>
  body?: string | Uint8Array
  json?: Json
}

export interface HttpResponse {
  status: number
  ok: boolean
  headers: [string, string][]
  body: Uint8Array
  header(name: string): string | null
  text(): string
  json(): Json
}

export interface Http {
  send(request: HttpRequest): HttpResponse
  get(url: string): HttpResponse
}

export interface Notification {
  kind: string
  recipientUserId: string
  title: string
  body?: string
  record?: RecordRef
  link?: string
  data?: JsonObject
  dedupeKey?: string
}

export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error'

export interface RecordSnapshot {
  values: JsonObject
  custom_fields: JsonObject
}

export interface HookState {
  operation_id: string
  record_type: string
  record_id: string
  phase: string
  before: RecordSnapshot | null
  before_available: boolean
  proposed: RecordSnapshot | null
  persisted: RecordSnapshot | null
  effects_allowed: boolean
}

export interface ExecutionContext {
  id: string
  correlation_id: string
  parent_id: string | null
  event_id: string | null
  executor: string
  executor_kind: string
  automation: { automation_id: string; run_id: string; step_id: string } | null
  initiator: { kind: string; id: string | null } | null
  trigger: string
  chain: string[]
}

export type EffectCommand =
  | { kind: 'create'; record_type: string; values: JsonObject }
  | { kind: 'update'; record_type: string; id: string; values: JsonObject }
  | { kind: 'delete' | 'archive' | 'restore'; record_type: string; id: string }
  | { kind: 'kv_set'; collection: string; id: string; value: Json }
  | { kind: 'kv_delete'; collection: string; id: string }
  | { kind: 'emit'; name: string; record_type?: string | null; record_id?: string | null; payload: JsonObject }
  | { kind: 'notify'; name: string; recipient_user_id: string; title: string; body?: string | null; record_type?: string | null; record_id?: string | null; link?: string | null; data: JsonObject; dedupe_key?: string | null }
  | { kind: 'invoke'; operation: string; input: JsonObject }
  | { kind: 'email'; to: string; to_name?: string | null; subject: string; text: string; html?: string | null; purpose?: 'transactional' | 'marketing' }
  | { kind: 'http'; authentication?: string; on_response?: string; context?: Json; client_identity?: string; method: string; url: string; headers?: [string, string][]; body?: string | null }

export interface Ctx {
  readonly records: Records
  readonly http: Http
  kv(collection: string): Kv
  settings(): JsonObject
  setting<T extends Json = Json>(key: string): T | null
  secret(key: string): string | null
  can(permission: string): boolean
  permissions(): string[]
  notify(notification: Notification): void
  emit(name: string, record: RecordRef | null | undefined, payload?: JsonObject): void
  invoke<T extends Json = Json>(operation: string, input?: JsonObject): T
  effectResult(id: string): Json | null
  deferEffect(key: string, command: EffectCommand): string
  nextNumber(sequence: string): number
  setAuditReason(reason: string | null): void
  uploadFile(recordType: string, recordId: string, filename: string, body: Uint8Array | string): JsonObject
  readFile(fileId: string): { file: JsonObject; body: Uint8Array }
  respondBytes(contentType: string, filename: string | null, body: Uint8Array | string): void
  organization(): JsonObject
  listUsers(query?: JsonObject): JsonObject
  listRoles(): Json
  listTeams(): Json
  hookState(): HookState | null
  executionContext(): ExecutionContext | null
  now(): string
  log(level: LogLevel, message: string): void
  info(message: string): void
}

export declare const ctx: Ctx

export declare function respond(status: number, body?: Json): Response
export declare function ok(body?: Json): Response
export declare function created(body?: Json): Response

export interface PluginDefinition {
  onHook?(ctx: Ctx, call: HookCall): HookOutcome | void
  onEvent?(ctx: Ctx, event: PluginEvent): void
  onRoute?(ctx: Ctx, request: Request): Response | undefined | void
  onSchedule?(ctx: Ctx, name: string, slot: string): void
  onAction?(ctx: Ctx, call: ActionCall): Json | void
  onWidget?(ctx: Ctx, request: WidgetRequest): Json | void
  onOperation?(ctx: Ctx, key: string, input: Json): Json | void
  onContribution?(ctx: Ctx, point: string, name: string, input: Json): Json | void
}

export interface PluginExports {
  onHook: unknown
  onEvent: unknown
  onRoute: unknown
  onSchedule: unknown
  onAction: unknown
  onWidget: unknown
  onOperation: unknown
  onContribution: unknown
}

export declare function definePlugin(plugin: PluginDefinition): PluginExports
