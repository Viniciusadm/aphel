import { SlidersHorizontal } from 'lucide-react'
import type { ReactNode } from 'react'
import { Button } from './button'
import { Dialog, DialogClose, DialogContent, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from './dialog'
import { useUiLabels } from './labels-context'

export interface WorkspaceOptionsProps {
  label: string
  count?: number
  children: ReactNode
}

export function WorkspaceOptions({ label, count = 0, children }: WorkspaceOptionsProps) {
  const labels = useUiLabels()
  return <Dialog>
    <DialogTrigger asChild>
      <Button variant="outline" size="sm">
        <SlidersHorizontal aria-hidden />
        {label}
        {count > 0 && <span className="rounded-full bg-primary/10 px-1.5 text-xs tabular-nums">{count}</span>}
      </Button>
    </DialogTrigger>
    <DialogContent side="bottom" aria-describedby={undefined} className="md:inset-y-0 md:left-auto md:right-0 md:h-full md:max-h-none md:w-96 md:rounded-none md:border-l">
      <DialogHeader className="sticky top-0 z-10 border-b bg-background p-4 pr-14">
        <DialogTitle>{label}</DialogTitle>
      </DialogHeader>
      <div className="grid min-w-0 gap-5 p-4">{children}</div>
      <DialogFooter className="mx-0 mt-auto px-4 pb-[max(1rem,env(safe-area-inset-bottom))]">
        <DialogClose asChild><Button>{labels.close}</Button></DialogClose>
      </DialogFooter>
    </DialogContent>
  </Dialog>
}
