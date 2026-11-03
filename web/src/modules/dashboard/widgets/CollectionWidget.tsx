import { Slot } from '@/kernel/modules/Slot'
export interface CollectionData { sections: { key: string; data: unknown }[] }
export function CollectionWidget({ collection, data }: { collection: string; data: CollectionData }) {
  return <div className="grid gap-4 sm:grid-cols-2">{data.sections.map((section) => <Slot key={section.key} name="dashboard.source" props={{ collection, source: section.key, data: section.data }} />)}</div>
}
