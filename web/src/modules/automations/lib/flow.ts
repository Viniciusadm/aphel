import type { AutomationDefinition, BranchCaseSpec, StepSpec, StepType } from '../types'

export const DEFAULT_SLOT = '__default'
export const MATCHED_SLOT = '__matched'
export const TIMEOUT_SLOT = '__timeout'

export interface FlowBranch {
  slot: string
  nodes: FlowNode[]
}

export interface StepNode {
  kind: 'step'
  step: StepSpec
  children: FlowBranch[]
}

export interface JumpNode {
  kind: 'jump'
  uid: string
  target: string
}

export type FlowNode = StepNode | JumpNode

export interface InsertionPoint {
  path: SequencePath
  index: number
}

export interface SequenceAddress {
  stepId: string
  slot: string
}

export type SequencePath = SequenceAddress[]

let jumpSequence = 0

function jump(target: string): JumpNode {
  jumpSequence += 1
  return { kind: 'jump', uid: `jump-${jumpSequence}`, target }
}

export function isTerminal(type: StepType): boolean {
  return type === 'branch' || type === 'wait_until'
}

export function slotsOf(step: StepSpec): string[] {
  if (step.type === 'branch') return [...(step.cases ?? []).map((item) => item.key), DEFAULT_SLOT]
  if (step.type === 'wait_until') return [MATCHED_SLOT, TIMEOUT_SLOT]
  return []
}

export function definitionToFlow(definition: AutomationDefinition | null | undefined): FlowNode[] {
  const steps = definition?.steps ?? []
  const byId = new Map(steps.map((step) => [step.id, step]))
  const placed = new Set<string>()

  const sequence = (startId: string | null | undefined): FlowNode[] => {
    const nodes: FlowNode[] = []
    let current = startId ?? null
    while (current) {
      if (placed.has(current)) {
        nodes.push(jump(current))
        break
      }
      const step = byId.get(current)
      if (!step) break
      placed.add(current)
      const node: StepNode = { kind: 'step', step, children: [] }
      nodes.push(node)
      if (step.type === 'branch') {
        const cases = step.cases ?? []
        node.children = [
          ...cases.map((item) => ({ slot: item.key, nodes: sequence(item.next) })),
          { slot: DEFAULT_SLOT, nodes: sequence(step.default) },
        ]
        break
      }
      if (step.type === 'wait_until') {
        node.children = [
          { slot: MATCHED_SLOT, nodes: sequence(step.next) },
          { slot: TIMEOUT_SLOT, nodes: sequence(step.timeout_next) },
        ]
        break
      }
      current = step.next ?? null
    }
    return nodes
  }

  return sequence(definition?.start ?? steps[0]?.id)
}

function headOf(nodes: FlowNode[]): string | null {
  const first = nodes[0]
  if (!first) return null
  return first.kind === 'jump' ? first.target : first.step.id
}

function childNodes(node: StepNode, slot: string): FlowNode[] {
  return node.children.find((child) => child.slot === slot)?.nodes ?? []
}

function withoutLinks(step: StepSpec): StepSpec {
  const copy: StepSpec = { ...step }
  delete copy.next
  delete copy.default
  delete copy.timeout_next
  return copy
}

function compact<T extends object>(value: T): T {
  return Object.fromEntries(
    Object.entries(value).filter(([, item]) => item !== undefined && item !== null),
  ) as T
}

export function flowToDefinition(
  nodes: FlowNode[],
  options?: AutomationDefinition['options'],
): AutomationDefinition {
  const steps: StepSpec[] = []

  const emit = (sequence: FlowNode[]) => {
    sequence.forEach((node, index) => {
      if (node.kind === 'jump') return
      const next = headOf(sequence.slice(index + 1))
      const step = withoutLinks(node.step)
      if (step.type === 'branch') {
        step.cases = (step.cases ?? []).map(
          (item): BranchCaseSpec => compact({ ...item, next: headOf(childNodes(node, item.key)) }),
        )
        step.default = headOf(childNodes(node, DEFAULT_SLOT))
      } else if (step.type === 'wait_until') {
        step.next = headOf(childNodes(node, MATCHED_SLOT))
        step.timeout_next = headOf(childNodes(node, TIMEOUT_SLOT))
      } else {
        step.next = next
      }
      steps.push(compact(step))
      for (const child of node.children) emit(child.nodes)
    })
  }

  emit(nodes)
  const definition: AutomationDefinition = { steps }
  const start = headOf(nodes)
  if (start) definition.start = start
  if (options && Object.keys(options).length > 0) definition.options = options
  return definition
}

export function collectStepIds(nodes: FlowNode[], into: Set<string> = new Set()): Set<string> {
  for (const node of nodes) {
    if (node.kind === 'jump') continue
    into.add(node.step.id)
    for (const child of node.children) collectStepIds(child.nodes, into)
  }
  return into
}

export function collectSteps(nodes: FlowNode[], into: StepSpec[] = []): StepSpec[] {
  for (const node of nodes) {
    if (node.kind === 'jump') continue
    into.push(node.step)
    for (const child of node.children) collectSteps(child.nodes, into)
  }
  return into
}

function sanitizeId(base: string): string {
  const cleaned = base
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 56)
  return cleaned || 'step'
}

export function uniqueId(base: string, taken: Set<string>): string {
  const root = sanitizeId(base)
  if (!taken.has(root)) return root
  for (let suffix = 2; ; suffix += 1) {
    const candidate = `${root}_${suffix}`
    if (!taken.has(candidate)) return candidate
  }
}

function mapSequence(
  nodes: FlowNode[],
  path: SequencePath,
  update: (nodes: FlowNode[]) => FlowNode[],
): FlowNode[] {
  if (path.length === 0) return update(nodes)
  const [head, ...rest] = path
  return nodes.map((node) => {
    if (node.kind !== 'step' || node.step.id !== head.stepId) return node
    return {
      ...node,
      children: node.children.map((child) =>
        child.slot === head.slot ? { ...child, nodes: mapSequence(child.nodes, rest, update) } : child,
      ),
    }
  })
}

function prune(nodes: FlowNode[], ids: Set<string>): FlowNode[] {
  return nodes
    .filter((node) => node.kind === 'step' || ids.has(node.target))
    .map((node) =>
      node.kind === 'jump'
        ? node
        : { ...node, children: node.children.map((child) => ({ ...child, nodes: prune(child.nodes, ids) })) },
    )
}

function pruneJumps(nodes: FlowNode[]): FlowNode[] {
  return prune(nodes, collectStepIds(nodes))
}

export function insertStep(nodes: FlowNode[], point: InsertionPoint, step: StepSpec): FlowNode[] {
  return mapSequence(nodes, point.path, (sequence) => {
    const before = sequence.slice(0, point.index)
    const after = sequence.slice(point.index)
    const node: StepNode = { kind: 'step', step, children: [] }
    if (isTerminal(step.type)) {
      const slots = slotsOf(step)
      node.children = slots.map((slot, index) => ({ slot, nodes: index === 0 ? after : [] }))
      return [...before, node]
    }
    return [...before, node, ...after]
  })
}

function findStep(nodes: FlowNode[], stepId: string): StepNode | null {
  for (const node of nodes) {
    if (node.kind === 'jump') continue
    if (node.step.id === stepId) return node
    for (const child of node.children) {
      const found = findStep(child.nodes, stepId)
      if (found) return found
    }
  }
  return null
}

export function findStepNode(nodes: FlowNode[], stepId: string): StepNode | null {
  return findStep(nodes, stepId)
}

export function hasDescendants(node: StepNode): boolean {
  return node.children.some((child) => child.nodes.length > 0)
}

function removeFrom(nodes: FlowNode[], stepId: string): FlowNode[] {
  return nodes
    .filter((node) => node.kind === 'jump' || node.step.id !== stepId)
    .map((node) =>
      node.kind === 'jump'
        ? node
        : { ...node, children: node.children.map((child) => ({ ...child, nodes: removeFrom(child.nodes, stepId) })) },
    )
}

export function removeStep(nodes: FlowNode[], stepId: string): FlowNode[] {
  return pruneJumps(removeFrom(nodes, stepId))
}

export function removeJump(nodes: FlowNode[], uid: string): FlowNode[] {
  const visit = (sequence: FlowNode[]): FlowNode[] =>
    sequence
      .filter((node) => node.kind === 'step' || node.uid !== uid)
      .map((node) =>
        node.kind === 'jump'
          ? node
          : { ...node, children: node.children.map((child) => ({ ...child, nodes: visit(child.nodes) })) },
      )
  return visit(nodes)
}

function syncChildren(node: StepNode, step: StepSpec): FlowBranch[] {
  const slots = slotsOf(step)
  return slots.map((slot) => node.children.find((child) => child.slot === slot) ?? { slot, nodes: [] })
}

export function updateStep(nodes: FlowNode[], stepId: string, step: StepSpec): FlowNode[] {
  const visit = (sequence: FlowNode[]): FlowNode[] =>
    sequence.map((node) => {
      if (node.kind === 'jump') return node
      if (node.step.id === stepId) return { ...node, step, children: syncChildren(node, step) }
      return { ...node, children: node.children.map((child) => ({ ...child, nodes: visit(child.nodes) })) }
    })
  return pruneJumps(visit(nodes))
}

export function renameCaseSlot(nodes: FlowNode[], stepId: string, from: string, to: string): FlowNode[] {
  const visit = (sequence: FlowNode[]): FlowNode[] =>
    sequence.map((node) => {
      if (node.kind === 'jump') return node
      if (node.step.id === stepId) {
        return {
          ...node,
          children: node.children.map((child) => (child.slot === from ? { ...child, slot: to } : child)),
        }
      }
      return { ...node, children: node.children.map((child) => ({ ...child, nodes: visit(child.nodes) })) }
    })
  return visit(nodes)
}

export function stepIndexMap(definition: AutomationDefinition): string[] {
  return definition.steps.map((step) => step.id)
}
