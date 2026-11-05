import * as host from 'aphel:plugin/host@1.0.0'

export const hooks = Object.freeze({
  SAVING: 'record.saving',
  VALIDATING: 'record.validating',
  SAVED: 'record.saved',
  DELETING: 'record.deleting',
  ARCHIVING: 'record.archiving',
  COMMITTED: 'record.committed',
  DELETED: 'record.deleted',
  RESTORED: 'record.restored',
  PURGED: 'record.purged',
})

export class PluginError extends Error {
  constructor(code, message, { status, fields } = {}) {
    super(message)
    this.code = code
    this.status = status
    this.fields = fields ?? []
  }

  static field(code, field, message) {
    return new PluginError(code, message, { fields: [{ field, message }] })
  }

  static notFound(code, message) {
    return new PluginError(code, message, { status: 404 })
  }

  static forbidden(code, message) {
    return new PluginError(code, message, { status: 403 })
  }

  get payload() {
    return {
      code: this.code,
      message: this.message,
      status: this.status ?? undefined,
      fields: this.fields.map(({ field, message }) => ({ field, message })),
    }
  }
}

function hostError(error) {
  const payload = error?.payload
  if (payload && typeof payload.code === 'string') {
    return new PluginError(payload.code, payload.message, { status: payload.status, fields: payload.fields })
  }
  return error
}

function call(fn) {
  try {
    return fn()
  } catch (error) {
    throw hostError(error)
  }
}

function parse(json, fallback = null) {
  try {
    return JSON.parse(json)
  } catch {
    return fallback
  }
}

function pairs(list) {
  return Object.fromEntries(list)
}

function actor(raw) {
  return { kind: raw.kind.replace('-', '_'), id: raw.id ?? null, userId: raw.userId ?? null }
}

const encoder = new TextEncoder()
const decoder = new TextDecoder()

class Kv {
  constructor(collection) {
    this.collection = collection
  }

  get(id) {
    const value = call(() => host.kvGet(this.collection, id))
    return value === undefined || value === null ? null : parse(value)
  }

  set(id, value) {
    call(() => host.kvSet(this.collection, id, JSON.stringify(value ?? null)))
  }

  delete(id) {
    call(() => host.kvDelete(this.collection, id))
  }

  list() {
    return call(() => host.kvList(this.collection)).map(([id, value]) => ({ id, value: parse(value) }))
  }

  increment(id, delta = 1) {
    return Number(call(() => host.kvIncrement(this.collection, id, BigInt(delta))))
  }

  setIfAbsent(id, value) {
    return call(() => host.kvSetIfAbsent(this.collection, id, JSON.stringify(value ?? null)))
  }
}

function listQuery({ filter, sort, search, page, perPage } = {}) {
  return {
    filterJson: filter === undefined ? undefined : JSON.stringify(filter),
    sort,
    search,
    page,
    perPage,
  }
}

const records = Object.freeze({
  get(recordType, id) {
    const record = call(() => host.getRecord(recordType, id))
    return record === undefined || record === null ? null : parse(record)
  },
  list(recordType, options) {
    const result = call(() => host.listRecords(recordType, listQuery(options)))
    return { data: result.recordsJson.map((json) => parse(json)), total: Number(result.total) }
  },
  create(recordType, values) {
    return parse(call(() => host.createRecord(recordType, JSON.stringify(values ?? {}))))
  },
  update(recordType, id, values) {
    return parse(call(() => host.updateRecord(recordType, id, JSON.stringify(values ?? {}))))
  },
  delete(recordType, id) {
    call(() => host.deleteRecord(recordType, id))
  },
  updateIf(recordType, id, expectedUpdatedAt, values) {
    return parse(
      call(() => host.updateRecordIf(recordType, id, expectedUpdatedAt, JSON.stringify(values ?? {}))),
    )
  },
  apply(mutations) {
    const list = mutations.map(({ recordType, id, expectedUpdatedAt, values }) => ({
      recordType,
      id: id ?? undefined,
      expectedUpdatedAt: expectedUpdatedAt ?? undefined,
      valuesJson: JSON.stringify(values ?? {}),
    }))
    return call(() => host.applyRecords(list)).map((json) => parse(json))
  },
  aggregate(query) {
    return parse(call(() => host.aggregateRecords(JSON.stringify(query))))
  },
  iterate(recordType, options, cursor) {
    const [list, next] = call(() => host.iterateRecords(recordType, listQuery(options), cursor ?? undefined))
    return { data: list.map((json) => parse(json)), cursor: next ?? null }
  },
  ref(index) {
    return { $ref: index }
  },
})

const http = Object.freeze({
  send({ method = 'GET', url, headers = {}, body, json } = {}) {
    const headerList = Object.entries(headers).map(([name, value]) => [name, String(value)])
    let bytes
    if (json !== undefined) {
      bytes = encoder.encode(JSON.stringify(json))
      headerList.push(['content-type', 'application/json'])
    } else if (typeof body === 'string') {
      bytes = encoder.encode(body)
    } else if (body !== undefined) {
      bytes = body
    }
    const response = call(() => host.httpFetch({ method, url, headers: headerList, body: bytes }))
    const responseBytes = new Uint8Array(response.body)
    return {
      status: response.status,
      ok: response.status >= 200 && response.status < 300,
      headers: response.headers,
      header(name) {
        const lower = name.toLowerCase()
        return response.headers.find(([key]) => key.toLowerCase() === lower)?.[1] ?? null
      },
      body: responseBytes,
      text() {
        return decoder.decode(responseBytes)
      },
      json() {
        return JSON.parse(decoder.decode(responseBytes))
      },
    }
  },
  get(url) {
    return http.send({ method: 'GET', url })
  },
})

export const ctx = Object.freeze({
  records,
  http,
  kv(collection) {
    return new Kv(collection)
  },
  settings() {
    return parse(host.settingsJson(), {})
  },
  setting(key) {
    return this.settings()[key] ?? null
  },
  secret(key) {
    return call(() => host.secret(key)) ?? null
  },
  can(permission) {
    return host.can(permission)
  },
  permissions() {
    return host.permissions()
  },
  notify({ kind, recipientUserId, title, body, record, link, data, dedupeKey }) {
    call(() =>
      host.notify({
        kind,
        recipientUserId,
        title,
        body,
        recordType: record?.type,
        recordId: record?.id,
        link,
        dataJson: JSON.stringify(data ?? {}),
        dedupeKey,
      }),
    )
  },
  emit(name, record, payload) {
    call(() => host.emit(name, record?.type, record?.id, JSON.stringify(payload ?? {})))
  },
  deferEffect(key, command) {
    return call(() => host.deferEffect(key, JSON.stringify(command)))
  },
  nextNumber(sequence) {
    return Number(call(() => host.nextNumber(sequence)))
  },
  setAuditReason(reason) {
    host.setAuditReason(reason ?? undefined)
  },
  uploadFile(recordType, recordId, filename, body) {
    const bytes = typeof body === 'string' ? encoder.encode(body) : body
    return parse(call(() => host.uploadFile(recordType, recordId, filename, bytes)))
  },
  readFile(fileId) {
    const [file, body] = call(() => host.readFile(fileId))
    return { file: parse(file), body: new Uint8Array(body) }
  },
  respondBytes(contentType, filename, body) {
    const bytes = typeof body === 'string' ? encoder.encode(body) : body
    call(() => host.respondBytes(contentType, filename ?? undefined, bytes))
  },
  organization() {
    return parse(call(() => host.organizationJson()))
  },
  listUsers(query) {
    return parse(call(() => host.listUsers(JSON.stringify(query ?? {}))))
  },
  listRoles() {
    return parse(call(() => host.listRoles()))
  },
  listTeams() {
    return parse(call(() => host.listTeams()))
  },
  hookState() {
    return parse(host.hookStateJson())
  },
  executionContext() {
    return parse(host.executionContextJson())
  },
  invoke(operation, input) {
    return JSON.parse(call(() => host.invoke(operation, JSON.stringify(input ?? {}))))
  },
  now() {
    return host.now()
  },
  log(level, message) {
    host.log(level, String(message))
  },
  info(message) {
    host.log('info', String(message))
  },
})

export function respond(status, body) {
  return { status, body }
}

export const ok = (body) => respond(200, body)
export const created = (body) => respond(201, body)

function guard(fn) {
  try {
    return fn()
  } catch (error) {
    if (error instanceof PluginError) throw error
    throw new PluginError('plugin_error', error instanceof Error ? error.message : String(error))
  }
}

function routeNotFound(request) {
  return new PluginError('route_not_found', `no route for ${request.method} ${request.path}`, { status: 404 })
}

export function definePlugin(plugin) {
  return {
    onHook(raw) {
      return guard(() => {
        const call = {
          hook: raw.hook,
          recordType: raw.recordType,
          recordId: raw.recordId,
          mode: raw.mode ?? null,
          actor: actor(raw.actor),
          changes: raw.changes.map((change) => ({
            field: change.field,
            old: parse(change.oldJson),
            new: parse(change.newJson),
          })),
          values: parse(raw.valuesJson, {}),
          customFields: parse(raw.customFieldsJson, {}),
          is(hook, recordType) {
            return this.hook === hook && this.recordType === recordType
          },
          changed(field) {
            return this.changes.find((change) => change.field === field) ?? null
          },
          customChanged(field) {
            return this.changed(`custom_fields.${field}`)
          },
          get isCreate() {
            return this.mode === 'create'
          },
        }
        const outcome = plugin.onHook?.(ctx, call) ?? {}
        return {
          valuesJson: JSON.stringify(outcome.values ?? {}),
          customFieldsJson: JSON.stringify(outcome.customFields ?? {}),
        }
      })
    },
    onEvent(raw) {
      guard(() =>
        plugin.onEvent?.(ctx, {
          id: raw.id,
          type: raw.eventType,
          recordType: raw.recordType ?? null,
          recordId: raw.recordId ?? null,
          actor: actor(raw.actor),
          occurredAt: raw.occurredAt,
          payload: parse(raw.payloadJson),
        }),
      )
    },
    onRoute(raw) {
      return guard(() => {
        const request = {
          method: raw.method,
          path: raw.path,
          route: raw.route,
          params: pairs(raw.params),
          query: pairs(raw.query),
          headers: raw.headers,
          body: parse(raw.bodyJson),
          actor: actor(raw.actor),
          is(method, route) {
            return this.method === method && this.route === route
          },
          header(name) {
            const lower = name.toLowerCase()
            return this.headers.find(([key]) => key === lower)?.[1] ?? null
          },
        }
        if (!plugin.onRoute) throw routeNotFound(request)
        const response = plugin.onRoute(ctx, request)
        if (!response) throw routeNotFound(request)
        return { status: response.status ?? 200, bodyJson: JSON.stringify(response.body ?? null) }
      })
    },
    onSchedule(name, slot) {
      guard(() => plugin.onSchedule?.(ctx, name, slot))
    },
    onAction(raw) {
      return guard(() => {
        const output = plugin.onAction?.(ctx, {
          name: raw.name,
          input: parse(raw.inputJson, {}),
          recordType: raw.recordType,
          recordId: raw.recordId,
          record: parse(raw.recordJson),
          automationId: raw.automationId,
          runId: raw.runId,
          actor: actor(raw.actor),
        })
        return JSON.stringify(output ?? null)
      })
    },
    onWidget(name, raw) {
      return guard(() => {
        const data = plugin.onWidget?.(ctx, {
          name,
          settings: parse(raw.settingsJson, {}),
          periodFrom: raw.periodFrom,
          periodTo: raw.periodTo,
          timezone: raw.timezone,
          actor: actor(raw.actor),
        })
        return JSON.stringify(data ?? null)
      })
    },
    onOperation(key, inputJson) {
      return guard(() => {
        const output = plugin.onOperation?.(ctx, key, parse(inputJson))
        return JSON.stringify(output ?? null)
      })
    },
    onContribution(point, name, inputJson) {
      return guard(() => {
        const output = plugin.onContribution?.(ctx, point, name, parse(inputJson))
        return JSON.stringify(output ?? null)
      })
    },
  }
}
