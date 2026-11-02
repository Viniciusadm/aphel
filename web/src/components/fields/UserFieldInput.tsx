import { X } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Avatar } from '@/components/ui/avatar'
import { Button } from '@/components/ui/button'
import { useOrganizationUsers } from '@/features/records/api'

import { SearchablePicker } from './SearchablePicker'
import type { FieldInputBaseProps } from './types'

export interface UserFieldInputProps extends FieldInputBaseProps<string> {
  clearable?: boolean
}

function UserLabel({ name }: { name: string }) {
  return (
    <span className="inline-flex min-w-0 items-center gap-2">
      <Avatar name={name} />
      <span className="truncate">{name}</span>
    </span>
  )
}

export function UserFieldInput({ clearable = true, ...props }: UserFieldInputProps) {
  const { t } = useTranslation('records-form')
  const users = useOrganizationUsers()
  const [open, setOpen] = useState(false)
  const [search, setSearch] = useState('')
  const all = users.data ?? []
  const term = search.trim().toLocaleLowerCase()
  const visible = term ? all.filter((user) => user.display_name.toLocaleLowerCase().includes(term)) : all
  const current = props.value ? all.find((user) => user.id === props.value) : undefined

  return (
    <div className="flex min-w-0 items-center gap-1">
      <SearchablePicker
        id={props.id}
        open={open}
        onOpenChange={setOpen}
        search={search}
        onSearchChange={setSearch}
        selected={props.value ? [props.value] : []}
        onSelect={(value) => props.onChange(value)}
        options={visible.map((user) => ({
          value: user.id,
          label: user.display_name,
          content: <UserLabel name={user.display_name} />,
        }))}
        loading={users.isPending}
        disabled={props.disabled}
        invalid={props.invalid}
        autoFocus={props.autoFocus}
        className={props.className}
        onBlur={props.onBlur}
        aria-describedby={props['aria-describedby']}
        aria-labelledby={props['aria-labelledby']}
        aria-label={props['aria-label']}
      >
        {props.value ? (
          <UserLabel name={current?.display_name ?? (users.isPending ? props.value : t('input.unknownUser'))} />
        ) : (
          <span className="text-muted-foreground">{t('input.placeholder')}</span>
        )}
      </SearchablePicker>
      {clearable && props.value && !props.disabled && (
        <Button variant="ghost" size="icon" aria-label={t('input.clear')} onClick={() => props.onChange(null)}>
          <X />
        </Button>
      )}
    </div>
  )
}
