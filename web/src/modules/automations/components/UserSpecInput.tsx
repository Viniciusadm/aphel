import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { activeFields, type FieldSchema, NativeSelect, useOrganizationUsers } from '@/sdk'
import type { JsonObject } from '../types'

export type UserSpecType = 'record_owner' | 'user' | 'field'

export interface UserSpecInputProps {
  id?: string
  value: unknown
  onChange: (value: JsonObject) => void
  schema: FieldSchema | undefined
  invalid?: boolean
}

function specOf(value: unknown): JsonObject {
  return typeof value === 'object' && value !== null && !Array.isArray(value) ? (value as JsonObject) : {}
}

export function UserSpecInput({ id, value, onChange, schema, invalid }: UserSpecInputProps) {
  const { t } = useTranslation('automations')
  const { me } = useSession()
  const users = useOrganizationUsers()
  const spec = specOf(value)
  const type = (typeof spec.type === 'string' ? spec.type : 'record_owner') as UserSpecType
  const userFields = schema ? activeFields(schema).filter((field) => field.field_type === 'user') : []
  const hasOwner = userFields.some((field) => field.key === 'owner_id')

  const changeType = (next: UserSpecType) => {
    if (next === 'record_owner') onChange({ type: next })
    else if (next === 'user') onChange({ type: next, user_id: me.user.id })
    else onChange({ type: next, field: userFields[0]?.key ?? '' })
  }

  return (
    <div className="flex min-w-0 flex-wrap gap-2">
      <NativeSelect
        id={id}
        value={type}
        onChange={(event) => changeType(event.target.value as UserSpecType)}
        aria-invalid={invalid ? true : undefined}
        aria-label={t('userSpec.type')}
        className="min-w-40 flex-1"
      >
        {(hasOwner || type === 'record_owner') && <option value="record_owner">{t('userSpec.recordOwner')}</option>}
        <option value="user">{t('userSpec.user')}</option>
        {userFields.length > 0 && <option value="field">{t('userSpec.field')}</option>}
      </NativeSelect>
      {type === 'user' && (
        <NativeSelect
          value={typeof spec.user_id === 'string' ? spec.user_id : ''}
          onChange={(event) => onChange({ type, user_id: event.target.value })}
          aria-label={t('userSpec.user')}
          className="min-w-40 flex-1"
        >
          <option value="" disabled>
            {t('userSpec.pickUser')}
          </option>
          {(users.data ?? []).map((user) => (
            <option key={user.id} value={user.id}>
              {user.id === me.user.id ? t('userSpec.me', { name: user.display_name }) : user.display_name}
            </option>
          ))}
        </NativeSelect>
      )}
      {type === 'field' && (
        <NativeSelect
          value={typeof spec.field === 'string' ? spec.field : ''}
          onChange={(event) => onChange({ type, field: event.target.value })}
          aria-label={t('userSpec.field')}
          className="min-w-40 flex-1"
        >
          {userFields.map((field) => (
            <option key={field.key} value={field.key}>
              {field.label}
            </option>
          ))}
        </NativeSelect>
      )}
    </div>
  )
}
