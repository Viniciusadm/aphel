import { KeyboardSensor, MouseSensor, TouchSensor, useSensor, useSensors } from '@dnd-kit/core'

export const MOUSE_DRAG_DISTANCE = 6
export const TOUCH_DRAG_DELAY_MS = 250
export const TOUCH_DRAG_TOLERANCE = 5

export function useBoardSensors() {
  return useSensors(
    useSensor(MouseSensor, { activationConstraint: { distance: MOUSE_DRAG_DISTANCE } }),
    useSensor(TouchSensor, { activationConstraint: { delay: TOUCH_DRAG_DELAY_MS, tolerance: TOUCH_DRAG_TOLERANCE } }),
    useSensor(KeyboardSensor),
  )
}
