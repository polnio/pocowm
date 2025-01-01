use crate::layout::{LayoutElement, LayoutType};
use crate::window::Window;
use crate::PocoWM;
use bitflags::bitflags;
use smithay::backend::input::{
    AbsolutePositionEvent as _, Axis, ButtonState, Event as _, InputBackend, InputEvent, KeyState,
    KeyboardKeyEvent as _, PointerAxisEvent as _, PointerButtonEvent as _,
};
use smithay::input::keyboard;
use smithay::input::pointer;
use smithay::utils::SERIAL_COUNTER;

bitflags! {
    struct KeyModifiers: u8 {
        const CTRL = 1;
        const SHIFT = 2;
        const ALT = 4;
        const SUPER = 8;
    }
}

impl From<&keyboard::ModifiersState> for KeyModifiers {
    fn from(value: &keyboard::ModifiersState) -> Self {
        let mut modifiers = KeyModifiers::empty();
        if value.ctrl {
            modifiers |= KeyModifiers::CTRL;
        }
        if value.alt {
            modifiers |= KeyModifiers::ALT;
        }
        if value.shift {
            modifiers |= KeyModifiers::SHIFT;
        }
        if value.logo {
            modifiers |= KeyModifiers::SUPER;
        }
        modifiers
    }
}

impl PocoWM {
    pub(crate) fn handle_input<B: InputBackend>(&mut self, event: InputEvent<B>) -> Option<()> {
        match event {
            InputEvent::Keyboard { event } => {
                let serial = SERIAL_COUNTER.next_serial();
                let time = event.time_msec();
                let event_state = event.state();
                self.seat.get_keyboard().map(|keyboard| {
                    keyboard.input::<(), _>(
                        self,
                        event.key_code(),
                        event_state,
                        serial,
                        time,
                        |state, modifiers, key| {
                            match event_state {
                                KeyState::Pressed => {
                                    state.pressed_keys.insert(key.modified_sym());
                                }
                                KeyState::Released => {
                                    state.pressed_keys.remove(&key.modified_sym());
                                }
                            }
                            if event_state != KeyState::Pressed {
                                return keyboard::FilterResult::Forward;
                            }
                            let modifiers = KeyModifiers::from(modifiers);
                            if !modifiers.contains(KeyModifiers::ALT) {
                                return keyboard::FilterResult::Forward;
                            }
                            let syms = key.raw_syms();
                            if syms.contains(&keyboard::Keysym::Return) {
                                let _ = std::process::Command::new("kitty")
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .spawn();
                                return keyboard::FilterResult::Intercept(());
                            }
                            if syms.contains(&keyboard::Keysym::e) {
                                keyboard
                                    .current_focus()
                                    .and_then(|w| if w.state().is_empty() { Some(w) } else { None })
                                    .and_then(|w| state.layout.get_window_positions(&w))
                                    .map(|positions| {
                                        let mut layout = &mut state.layout;
                                        for pos in positions {
                                            match layout.elements.get_mut(pos) {
                                                Some(LayoutElement::SubLayout(sl)) => {
                                                    layout = sl;
                                                }
                                                _ => break,
                                            }
                                        }
                                        layout.layout_type = match layout.layout_type {
                                            LayoutType::Horizontal => LayoutType::Vertical,
                                            LayoutType::Vertical => LayoutType::Horizontal,
                                            LayoutType::Tabbed => LayoutType::Horizontal,
                                        };
                                        state.renderer.render(&state.layout);
                                    });
                                return keyboard::FilterResult::Intercept(());
                            }

                            if syms.contains(&keyboard::Keysym::b) {
                                state.switch_to_layout(LayoutType::Vertical);
                                return keyboard::FilterResult::Intercept(());
                            }
                            if syms.contains(&keyboard::Keysym::n) {
                                state.switch_to_layout(LayoutType::Horizontal);
                                return keyboard::FilterResult::Intercept(());
                            }
                            if syms.contains(&keyboard::Keysym::f) {
                                state.toggle_floating();
                                return keyboard::FilterResult::Intercept(());
                            }
                            keyboard::FilterResult::Forward
                        },
                    )
                });
            }
            InputEvent::PointerMotion { .. } => {}
            InputEvent::PointerMotionAbsolute { event, .. } => {
                let output = self.renderer.outputs().next()?;
                let output_geometry = self.renderer.output_geometry(output)?;
                let pos =
                    event.position_transformed(output_geometry.size) + output_geometry.loc.to_f64();
                let serial = SERIAL_COUNTER.next_serial();
                let pointer = self.seat.get_pointer()?;
                if !pointer.is_grabbed() {
                    let pointed_window = self.renderer.element_under(pos).map(|(w, _)| w.clone());
                    let focused_window = self.seat.get_keyboard().and_then(|k| k.current_focus());
                    Option::zip(pointed_window, focused_window).map(
                        |(pointed_window, focused_window)| {
                            if pointed_window != focused_window {
                                self.focus_window(Some(&pointed_window));
                            }
                        },
                    );
                }

                pointer.motion(
                    self,
                    self.renderer
                        .element_under(pos)
                        .map(|(w, p)| (w.clone(), p.to_f64())),
                    &pointer::MotionEvent {
                        location: pos,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerButton { event, .. } => {
                let pointer = self.seat.get_pointer()?;
                let button_state = event.state();
                let serial = SERIAL_COUNTER.next_serial();
                match button_state {
                    ButtonState::Pressed if pointer.is_grabbed() => {
                        self.layout.iter_windows().for_each(|window| {
                            window.set_activated(false);
                            window.toplevel().map(|t| t.send_pending_configure());
                        });
                        self.focus_window(None);
                    }
                    ButtonState::Pressed => {
                        let (window, _location) =
                            self.renderer.element_under(pointer.current_location())?;
                        self.focus_window(Some(&window.clone()));
                        self.renderer.elements().for_each(|window| {
                            window.toplevel().map(|t| t.send_pending_configure());
                        });
                    }
                    ButtonState::Released => {}
                }

                pointer.button(
                    self,
                    &pointer::ButtonEvent {
                        button: event.button_code(),
                        state: button_state,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerAxis { event } => {
                let pointer = self.seat.get_pointer()?;
                let mut frame = pointer::AxisFrame::new(event.time_msec());
                frame = self.handle_axis::<B>(frame, &event, Axis::Horizontal);
                frame = self.handle_axis::<B>(frame, &event, Axis::Vertical);
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            _ => {}
        }

        Some(())
    }

    pub fn handle_axis<B: InputBackend>(
        &mut self,
        mut frame: pointer::AxisFrame,
        event: &B::PointerAxisEvent,
        axis: Axis,
    ) -> pointer::AxisFrame {
        let amount = event
            .amount(axis)
            .unwrap_or_else(|| event.amount_v120(axis).unwrap_or_default() * 15.0 / 120.0);
        if amount != 0.0 {
            frame = frame.relative_direction(axis, event.relative_direction(axis));
            frame = frame.value(axis, amount);
            if let Some(discrete) = event.amount_v120(axis) {
                frame = frame.v120(axis, discrete as i32);
            }
        }
        if event.amount(axis) == Some(0.0) {
            frame = frame.stop(axis);
        }
        frame
    }

    pub fn focus_window(&mut self, window: Option<&Window>) {
        if let Some(window) = window.as_ref() {
            self.layout.iter_windows().for_each(Window::unfocus);
            window.focus();
        }
        self.seat.get_keyboard().map(|keyboard| {
            let serial = SERIAL_COUNTER.next_serial();
            keyboard.set_focus(self, window.cloned(), serial);
        });
    }
}
