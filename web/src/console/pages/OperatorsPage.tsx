import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link2, UserPlus, UserX } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { BadgeTone } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { ConfirmDialog } from '@/components/ui/confirm-dialog'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { QueryState } from '@/components/ui/query-state'
import { StatusBadge } from '@/components/ui/status-badge'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { consoleApi, consoleKeys, type Invitation, type Operator } from '../api'
import { InvitationLinkDialog } from '../components/InvitationLinkDialog'
import { InviteDialog } from '../components/InviteDialog'
import { notifyConsoleError, notifyConsoleSuccess } from '../errors'
import { formatDateTime } from '../format'
import { useConsoleSession } from '../session'

type OperatorStatus = 'invited' | 'active' | 'deactivated'

const TONES: Record<OperatorStatus, BadgeTone> = {
  invited: 'primary',
  active: 'success',
  deactivated: 'neutral',
}

export function OperatorsPage() {
  const { t } = useTranslation('console')
  const queryClient = useQueryClient()
  const session = useConsoleSession()
  const [inviting, setInviting] = useState(false)
  const [invitation, setInvitation] = useState<Invitation | null>(null)
  const [deactivating, setDeactivating] = useState<Operator | null>(null)
  const operators = useQuery({ queryKey: consoleKeys.operators, queryFn: consoleApi.operators })
  const refresh = () => void queryClient.invalidateQueries({ queryKey: consoleKeys.operators })
  const renew = useMutation({
    mutationFn: consoleApi.renewInvitation,
    onSuccess: setInvitation,
    onError: notifyConsoleError,
  })
  const deactivate = useMutation({
    mutationFn: consoleApi.deactivate,
    onSuccess: () => {
      refresh()
      notifyConsoleSuccess(t('operators.deactivated'))
    },
  })

  return (
    <Page>
      <PageHeader
        title={t('operators.title')}
        description={t('operators.description')}
        actions={
          <Button onClick={() => setInviting(true)}>
            <UserPlus className="size-4" aria-hidden />
            {t('operators.invite')}
          </Button>
        }
      />
      <QueryState query={operators}>
        {(data: Operator[]) => (
          <Card className="overflow-x-auto p-0">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{t('operators.name')}</TableHead>
                  <TableHead>{t('operators.status')}</TableHead>
                  <TableHead>{t('operators.lastLogin')}</TableHead>
                  <TableHead className="w-0" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.map((operator) => {
                  const status = operator.status as OperatorStatus
                  const self = operator.id === session.data?.id
                  return (
                    <TableRow key={operator.id}>
                      <TableCell>
                        <div className="font-medium">
                          {operator.display_name}
                          {self && <span className="ml-1 text-xs font-normal text-muted-foreground">({t('operators.you')})</span>}
                        </div>
                        <div className="text-xs text-muted-foreground">{operator.email}</div>
                      </TableCell>
                      <TableCell>
                        <StatusBadge tone={TONES[status] ?? 'neutral'}>{t(`operators.statuses.${status}`)}</StatusBadge>
                      </TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        {formatDateTime(operator.last_login_at) ?? t('operators.never')}
                      </TableCell>
                      <TableCell>
                        <div className="flex justify-end gap-1">
                          {status === 'invited' && (
                            <Button variant="ghost" size="sm" disabled={renew.isPending} onClick={() => renew.mutate(operator.id)}>
                              <Link2 className="size-4" aria-hidden />
                              {t('operators.renew')}
                            </Button>
                          )}
                          {status !== 'deactivated' && !self && (
                            <Button variant="ghost" size="sm" onClick={() => setDeactivating(operator)}>
                              <UserX className="size-4" aria-hidden />
                              {t('operators.deactivate')}
                            </Button>
                          )}
                        </div>
                      </TableCell>
                    </TableRow>
                  )
                })}
              </TableBody>
            </Table>
          </Card>
        )}
      </QueryState>
      <InviteDialog
        open={inviting}
        onOpenChange={setInviting}
        onInvited={(created) => {
          setInviting(false)
          setInvitation(created)
          refresh()
        }}
      />
      <InvitationLinkDialog invitation={invitation} onClose={() => setInvitation(null)} />
      <ConfirmDialog
        open={deactivating !== null}
        onOpenChange={(open) => !open && setDeactivating(null)}
        title={t('operators.deactivateTitle', { name: deactivating?.display_name ?? '' })}
        description={t('operators.deactivateDescription')}
        confirmLabel={t('operators.deactivate')}
        variant="destructive"
        pending={deactivate.isPending}
        onError={notifyConsoleError}
        onConfirm={async () => {
          if (deactivating) await deactivate.mutateAsync(deactivating.id)
          setDeactivating(null)
        }}
      />
    </Page>
  )
}
