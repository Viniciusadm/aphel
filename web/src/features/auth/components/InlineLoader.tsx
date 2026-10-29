import { LoaderCircle } from 'lucide-react'

export function InlineLoader({ label }: { label: string }) {
  return (
    <div role="status" className="flex items-center justify-center gap-2 py-6 text-sm text-muted-foreground">
      <LoaderCircle className="size-5 animate-spin" aria-hidden />
      <span>{label}</span>
    </div>
  )
}
