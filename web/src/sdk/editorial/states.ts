import { type BadgeTone } from '../ui'

import type { DocumentState } from './document'

export const STATE_TONES: Record<DocumentState, BadgeTone> = {
  draft: 'neutral',
  published: 'success',
  changed: 'warning',
  withdrawn: 'neutral',
}
