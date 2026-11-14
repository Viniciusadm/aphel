import { Inbox, MessageSquareReply, MessageSquareText, MessageSquareWarning, MessagesSquare, RadioTower } from 'lucide-react'
import { Navigate } from 'react-router'
import { type TimelineRenderer } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import { MANAGE_CHANNELS_PERMISSION, MESSAGING_KEY, VIEW_PERMISSION } from './api'
import { ConversationsTab } from './ConversationsTab'
import { ConversationEventContent, MessageEventContent } from './MessageEventContent'

const BASE_PATH = '/messaging'
import { ChannelsCard } from './IntegrationCard'
const CHANNELS_PATH = '/integrations/channels'

const conversationRenderer: TimelineRenderer = {
  icon: MessagesSquare,
  render: (item) => <ConversationEventContent item={item} />,
}

const EVENT_RENDERERS: Record<string, TimelineRenderer> = {
  'message.received': { icon: MessageSquareText, render: (item) => <MessageEventContent item={item} /> },
  'message.sent': { icon: MessageSquareReply, render: (item) => <MessageEventContent item={item} /> },
  'message.failed': { icon: MessageSquareWarning, render: (item) => <MessageEventContent item={item} /> },
  'conversation.assigned': conversationRenderer,
  'conversation.status_changed': conversationRenderer,
  'conversation.linked': conversationRenderer,
}

const messagingModule = defineModule({
  key: MESSAGING_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.keybinding({ id: 'messaging.send', label: 'keybindings.send', scope: 'editor', keys: ['mod+enter'] })
    app.routes([
      {
        path: 'messaging',
        children: [
          { index: true, lazy: async () => ({ Component: (await import('./InboxPage')).InboxPage }) },
          { path: 'channels', element: <Navigate to={CHANNELS_PATH} replace /> },
        ],
      },
    ])
    app.nav({ section: 'work',
      to: BASE_PATH,
      label: 'nav.inbox',
      icon: Inbox,
      end: true,
      permission: [VIEW_PERMISSION],
    })
    app.slot('integrations.cards', { key: 'channels', order: 300, permission: [MANAGE_CHANNELS_PERMISSION], render: ChannelsCard })
    app.settings('integrations', {
      group: 'connections',
      path: 'channels',
      label: 'nav.channels',
      description: 'channels.menuDescription',
      icon: RadioTower,
      permission: MANAGE_CHANNELS_PERMISSION,
      lazy: async () => ({ Component: (await import('./ChannelsPage')).ChannelsPage }),
    })
    app.recordSection(['people.person', 'deals.deal'], {
      key: `${MESSAGING_KEY}.conversations`,
      group: 'communication',
      label: 'tab.label',
      icon: MessagesSquare,
      permission: [VIEW_PERMISSION],
      render: ConversationsTab,
    })
    for (const [eventType, renderer] of Object.entries(EVENT_RENDERERS)) app.eventRenderer(eventType, renderer)
  },
})

export default messagingModule
