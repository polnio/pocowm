use super::decorations::{self, DECORATIONS_HEIGHT};
use super::{Window, WindowState};
use crate::utils::Edge;
use crate::PocoWM;
use smithay::backend::input::{ButtonState, KeyState};
use smithay::input::keyboard::{KeyboardTarget, Keysym, KeysymHandle};
use smithay::input::pointer::{self, PointerTarget};
use smithay::input::touch::TouchTarget;
use smithay::input::Seat;
use smithay::input::{keyboard, touch};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::Serial;
use smithay::wayland::seat::WaylandFocus;
use std::borrow::Cow;

// const RESIZE_GRAB_SIZE: u32 = BUTTON_GAP;
const RESIZE_GRAB_SIZE: u32 = 0;

impl WaylandFocus for Window {
    #[inline]
    fn wl_surface(&self) -> Option<Cow<WlSurface>> {
        self.inner().wl_surface()
    }
}

impl PointerTarget<PocoWM> for Window {
    fn enter(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, event: &pointer::MotionEvent) {
        self.seat_data_mut().pointer_location = Some(event.location);
        if self.decorations().is_some() && event.location.y < DECORATIONS_HEIGHT as f64 {
            return;
        }
        if let Some(wl_surface) = self.wl_surface() {
            let mut event = event.clone();
            if self.decorations().is_some() {
                event.location.y -= DECORATIONS_HEIGHT as f64;
            }
            PointerTarget::<PocoWM>::enter(wl_surface.as_ref(), seat, data, &event);
        }
    }

    fn motion(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, event: &pointer::MotionEvent) {
        self.seat_data_mut().pointer_location = Some(event.location);
        if self.decorations().is_some() && event.location.y < DECORATIONS_HEIGHT as f64 {
            return;
        }
        if let Some(wl_surface) = self.wl_surface() {
            let mut event = event.clone();
            if self.decorations().is_some() {
                event.location.y -= DECORATIONS_HEIGHT as f64;
            }
            PointerTarget::<PocoWM>::motion(wl_surface.as_ref(), seat, data, &event);
        }
    }

    fn relative_motion(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::RelativeMotionEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::relative_motion(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn button(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, event: &pointer::ButtonEvent) {
        let Some(mut loc) = self.seat_data().pointer_location else {
            return;
        };
        loc = loc - self.geometry().loc.to_f64();

        if event.state == ButtonState::Pressed && data.pressed_keys.contains(&Keysym::Alt_L) {
            const BTN_LEFT: u32 = 0x110;
            const BTN_RIGHT: u32 = 0x111;
            match event.button {
                BTN_LEFT => {
                    if let Some(surface) = self.toplevel().cloned() {
                        let seat = seat.clone();
                        let serial = event.serial;
                        data.loop_handle.insert_idle(move |state| {
                            state.xdg_move_request(&surface, &seat, serial);
                        });
                    }
                }
                BTN_RIGHT => {
                    if let Some(surface) = self.toplevel().cloned() {
                        let seat = seat.clone();
                        let serial = event.serial;
                        let edges = self.get_edge_under(loc);
                        data.loop_handle.insert_idle(move |state| {
                            state.xdg_resize_request(&surface, &seat, serial, edges);
                        });
                    }
                }
                _ => {}
            }
            return;
        }

        if event.state == ButtonState::Pressed {
            let mut edges = Edge::empty();
            if loc.x < RESIZE_GRAB_SIZE as f64 {
                edges |= Edge::LEFT;
            }
            if loc.y < RESIZE_GRAB_SIZE as f64 {
                edges |= Edge::TOP;
            }
            if loc.x > self.geometry().size.w as f64 - RESIZE_GRAB_SIZE as f64 {
                edges |= Edge::RIGHT;
            }
            if loc.y > self.geometry().size.h as f64 - RESIZE_GRAB_SIZE as f64 {
                edges |= Edge::BOTTOM;
            }
            if !edges.is_empty() {
                if let Some(xdg) = self.toplevel() {
                    let seat = seat.clone();
                    let xdg = xdg.clone();
                    let serial = event.serial;
                    data.loop_handle.insert_idle(move |data| {
                        data.xdg_resize_request(&xdg, &seat, serial, edges);
                    });
                    return;
                }
            }
        }

        if let Some(decorations) = self.decorations().as_ref() {
            if loc.y < DECORATIONS_HEIGHT as f64 {
                match decorations.get_button(loc) {
                    Some(decorations::Button::Close) => {
                        if event.state != ButtonState::Pressed {
                            return;
                        }
                        self.toplevel().map(|t| t.send_close());
                    }
                    Some(decorations::Button::Maximize) => {
                        if event.state != ButtonState::Pressed {
                            return;
                        }
                        if let Some(xdg) = self.toplevel().cloned() {
                            let is_maximized = self.state().contains(WindowState::MAXIMIZED);
                            data.loop_handle.insert_idle(move |data| {
                                if is_maximized {
                                    data.xdg_unmaximize_request(&xdg);
                                } else {
                                    data.xdg_maximize_request(&xdg);
                                }
                            });
                        }
                    }
                    Some(decorations::Button::Minimize) => {
                        if event.state != ButtonState::Pressed {
                            return;
                        }
                        if let Some(xdg) = self.toplevel().cloned() {
                            data.loop_handle.insert_idle(move |data| {
                                data.xdg_minimize_request(&xdg);
                            });
                        }
                    }
                    None => {
                        if let Some(xdg) = self.toplevel() {
                            // data.xdg_move_request(xdg, &seat.clone(), event.serial);
                            let seat = seat.clone();
                            let xdg = xdg.clone();
                            let serial = event.serial;
                            data.loop_handle.insert_idle(move |data| {
                                data.xdg_move_request(&xdg, &seat, serial);
                            });
                        }
                    }
                }
                return;
            }
        }
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::button(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn axis(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, frame: pointer::AxisFrame) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::axis(wl_surface.as_ref(), seat, data, frame);
        }
    }

    fn frame(&self, seat: &Seat<PocoWM>, data: &mut PocoWM) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::frame(wl_surface.as_ref(), seat, data);
        }
    }

    fn leave(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, serial: Serial, time: u32) {
        self.seat_data_mut().pointer_location = None;
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::leave(wl_surface.as_ref(), seat, data, serial, time);
        }
    }

    fn gesture_swipe_begin(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GestureSwipeBeginEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_swipe_begin(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn gesture_swipe_update(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GestureSwipeUpdateEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_swipe_update(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn gesture_swipe_end(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GestureSwipeEndEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_swipe_end(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn gesture_pinch_begin(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GesturePinchBeginEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_pinch_begin(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn gesture_pinch_update(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GesturePinchUpdateEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_pinch_update(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn gesture_pinch_end(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GesturePinchEndEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_pinch_end(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn gesture_hold_begin(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GestureHoldBeginEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_hold_begin(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn gesture_hold_end(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GestureHoldEndEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_hold_end(wl_surface.as_ref(), seat, data, event);
        }
    }
}

impl TouchTarget<PocoWM> for Window {
    fn down(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, event: &touch::DownEvent, seq: Serial) {
        if let Some(wl_surface) = self.wl_surface() {
            TouchTarget::<PocoWM>::down(wl_surface.as_ref(), seat, data, event, seq);
        }
    }

    fn up(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, event: &touch::UpEvent, seq: Serial) {
        if let Some(wl_surface) = self.wl_surface() {
            TouchTarget::<PocoWM>::up(wl_surface.as_ref(), seat, data, event, seq);
        }
    }

    fn motion(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &touch::MotionEvent,
        seq: Serial,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            TouchTarget::<PocoWM>::motion(wl_surface.as_ref(), seat, data, event, seq);
        }
    }

    fn frame(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, seq: Serial) {
        if let Some(wl_surface) = self.wl_surface() {
            TouchTarget::<PocoWM>::frame(wl_surface.as_ref(), seat, data, seq);
        }
    }

    fn cancel(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, seq: Serial) {
        if let Some(wl_surface) = self.wl_surface() {
            TouchTarget::<PocoWM>::cancel(wl_surface.as_ref(), seat, data, seq);
        }
    }

    fn shape(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &touch::ShapeEvent,
        seq: Serial,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            TouchTarget::<PocoWM>::shape(wl_surface.as_ref(), seat, data, event, seq);
        }
    }

    fn orientation(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &touch::OrientationEvent,
        seq: Serial,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            TouchTarget::<PocoWM>::orientation(wl_surface.as_ref(), seat, data, event, seq);
        }
    }
}

impl KeyboardTarget<PocoWM> for Window {
    fn enter(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        keys: Vec<KeysymHandle<'_>>,
        serial: Serial,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            KeyboardTarget::<PocoWM>::enter(wl_surface.as_ref(), seat, data, keys, serial);
        }
    }

    fn leave(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, serial: Serial) {
        if let Some(wl_surface) = self.wl_surface() {
            KeyboardTarget::<PocoWM>::leave(wl_surface.as_ref(), seat, data, serial);
        }
    }

    fn key(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        key: KeysymHandle<'_>,
        state: KeyState,
        serial: Serial,
        time: u32,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            KeyboardTarget::<PocoWM>::key(
                wl_surface.as_ref(),
                seat,
                data,
                key,
                state,
                serial,
                time,
            );
        }
    }

    fn modifiers(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        modifiers: keyboard::ModifiersState,
        serial: Serial,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            KeyboardTarget::<PocoWM>::modifiers(wl_surface.as_ref(), seat, data, modifiers, serial);
        }
    }
}
