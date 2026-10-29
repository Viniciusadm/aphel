export interface TimedEvent<T> {
  item: T
  start: number
  end: number
}

export interface PositionedEvent<T> extends TimedEvent<T> {
  column: number
  columns: number
}

export function layoutDay<T>(events: TimedEvent<T>[]): PositionedEvent<T>[] {
  const sorted = [...events].sort((left, right) => left.start - right.start || right.end - left.end)
  const result: PositionedEvent<T>[] = []
  let cluster: PositionedEvent<T>[] = []
  let columnEnds: number[] = []
  let clusterEnd = -1
  const flush = () => {
    for (const event of cluster) event.columns = columnEnds.length
    result.push(...cluster)
    cluster = []
    columnEnds = []
  }
  for (const event of sorted) {
    if (cluster.length > 0 && event.start >= clusterEnd) flush()
    let column = columnEnds.findIndex((end) => end <= event.start)
    if (column === -1) {
      column = columnEnds.length
      columnEnds.push(event.end)
    } else {
      columnEnds[column] = event.end
    }
    cluster.push({ ...event, column, columns: 1 })
    clusterEnd = cluster.length === 1 ? event.end : Math.max(clusterEnd, event.end)
  }
  flush()
  return result
}
