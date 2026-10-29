import { definePlugin } from '@aphel/plugin-sdk'

export default definePlugin({
  onRoute(ctx, request) {
    if (request.body.command) {
      const id = ctx.deferEffect('route-write', request.body.command)
      return { status: 202, body: { id, execution: ctx.executionContext() } }
    }
    return { body: { execution: ctx.executionContext(), hook: ctx.hookState() } }
  },
  onEvent(ctx, event) {
    ctx.deferEffect('event-write', { kind: 'kv_set', collection: 'events', id: event.id, value: ctx.executionContext() })
  },
})
