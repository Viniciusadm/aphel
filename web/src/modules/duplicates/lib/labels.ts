import type { ParseKeys, TFunction } from 'i18next'
import type { DuplicateTypeInfo } from '../api'

const RULE_KEYS: Record<string, ParseKeys<'duplicates'>> = {
  email: 'rules.email',
  phone: 'rules.phone',
  name_company: 'rules.name_company',
  domain: 'rules.domain',
  name: 'rules.name',
  identifier: 'rules.identifier',
}

const TYPE_KEYS: Record<string, ParseKeys<'duplicates'>> = {
  contact: 'recordTypes.contact',
  company: 'recordTypes.company',
  deal: 'recordTypes.deal',
  product: 'recordTypes.product',
}

export function typeLabel(t: TFunction<'duplicates'>, info: Pick<DuplicateTypeInfo, 'record_type' | 'label'>): string {
  const key = TYPE_KEYS[info.record_type]
  if (key) return t(key)
  return info.label ?? info.record_type
}

export function ruleLabelKey(rule: string): ParseKeys<'duplicates'> | undefined {
  return RULE_KEYS[rule]
}

export type DuplicatePermissions = DuplicateTypeInfo['permissions']
