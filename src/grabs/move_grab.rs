use crate::window::{Window, WindowState};
use crate::PocoWM;
use smithay::desktop::space::SpaceElement;
use smithay::input::pointer::{
    AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
    GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
    GestureSwipeUpdateEvent, GrabStartData, MotionEvent, PointerGrab, PointerInnerHandle,
    RelativeMotionEvent,
};
use smithay::utils::{Logical, Point};

pub struct MoveGrab {
    pub start_data: GrabStartData<PocoWM>,
    pub window: Window,
    pub initial_window_location: Point<i32, Logical>,
    pub new_location: Point<i32, Logical>,
    pub pointer_location: Point<f64, Logical>,
}

impl MoveGrab {
    pub fn unset_tiled(&mut self, data: &mut PocoWM) -> Option<()> {
        let neighbor = data.renderer.elements().find(|e| {
            if e == &&self.window {
                return false;
            }
            let Some(loc) = data.renderer.element_location(e) else {
                return false;
            };
            let mut rect = e.bbox();
            rect.loc += loc;
            rect.to_f64().contains(self.pointer_location)
        })?;
        let loc = data.renderer.element_location(neighbor)?;
        let edge = neighbor.get_edge_under(self.pointer_location - loc.to_f64());
        let old_id = data.layout.get_window_id(&self.window)?;
        let new_id = data.layout.get_window_id(neighbor)?;
        let el = data.layout.remove_element(old_id)?;
        data.layout.insert_element_at(new_id, edge, el);
        Some(())
    }
}

impl PointerGrab<PocoWM> for MoveGrab {
    fn motion(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        _focus: Option<(Window, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);
        let delta = event.location - self.start_data.location;
        let new_location = (self.initial_window_location.to_f64() + delta).to_i32_round();
        if self.window.state().contains(WindowState::FLOATING) {
            self.window.floating_rect_mut().loc = new_location;
        }
        data.renderer
            .map_element(self.window.clone(), new_location, true);
        self.new_location = new_location;
        self.pointer_location = event.location;
    }

    fn relative_motion(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        focus: Option<(Window, Point<f64, Logical>)>,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event);
    }

    fn button(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);

        // The button is a button code as defined in the
        // Linux kernel's linux/input-event-codes.h header file, e.g. BTN_LEFT.
        const BTN_LEFT: u32 = 0x110;
        if !handle.current_pressed().contains(&BTN_LEFT) {
            handle.unset_grab(self, data, event.serial, event.time, true);
        }
    }

    fn axis(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        details: AxisFrame,
    ) {
        handle.axis(data, details);
    }

    fn frame(&mut self, data: &mut PocoWM, handle: &mut PointerInnerHandle<'_, PocoWM>) {
        handle.frame(data);
    }

    fn gesture_swipe_begin(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event);
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event);
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event);
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event);
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event);
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event);
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event);
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event);
    }

    fn start_data(&self) -> &GrabStartData<PocoWM> {
        &self.start_data
    }

    fn unset(&mut self, data: &mut PocoWM) {
        let is_floating = self.window.state().contains(WindowState::FLOATING);
        if !is_floating {
            self.unset_tiled(data);
        }
        data.renderer.render(&data.layout);
    }
}
