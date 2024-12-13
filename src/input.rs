use std::borrow::Cow;

use crate::layout::{Layout, LayoutType};
use crate::PocoWM;
use bitflags::bitflags;
use smithay::backend::input::{
    AbsolutePositionEvent as _, ButtonState, Event, InputBackend, InputEvent, KeyState,
    KeyboardKeyEvent as _, PointerButtonEvent as _,
};
use smithay::desktop::Window;
use smithay::input::keyboard::{FilterResult, Keysym, ModifiersState};
use smithay::input::pointer::{ButtonEvent, MotionEvent};
use smithay::utils::SERIAL_COUNTER;
use smithay::wayland::seat::WaylandFocus as _;

bitflags! {
    struct KeyModifiers: u8 {
        const CTRL = 1;
        const SHIFT = 2;
        const ALT = 4;
        const SUPER = 8;
    }
}

impl From<&ModifiersState> for KeyModifiers {
    fn from(value: &ModifiersState) -> Self {
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
                            if event_state != KeyState::Pressed {
                                return FilterResult::Forward;
                            }
                            let modifiers = KeyModifiers::from(modifiers);
                            if !modifiers.contains(KeyModifiers::ALT) {
                                return FilterResult::Forward;
                            }
                            if key.raw_syms().contains(&Keysym::Return) {
                                let _ = std::process::Command::new("kitty")
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .spawn();
                                return FilterResult::Intercept(());
                            }
                            if key.raw_syms().contains(&Keysym::j) {
                                let focused_window = keyboard
                                    .current_focus()
                                    .and_then(|s| state.layout.get_window(&s))
                                    .cloned();
                                if let Some(focused_window) = focused_window {
                                    let mut layout = Layout::new(LayoutType::Vertical);
                                    let positions =
                                        state.layout.get_window_positions(&focused_window);
                                    state.layout.remove_window(&focused_window);
                                    layout.add_window(focused_window, None);
                                    state.layout.add_sublayout(layout, positions.as_deref());
                                } else if state.layout.elements.is_empty() {
                                    state.layout.layout_type = LayoutType::Vertical;
                                } else {
                                    let mut layout = Layout::new(LayoutType::Vertical);
                                    std::mem::swap(&mut state.layout, &mut layout);
                                    state.layout.add_sublayout(layout, None);
                                }
                                return FilterResult::Intercept(());
                            }
                            FilterResult::Forward
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

                let under = self.renderer.surface_under(pos);
                pointer.motion(
                    self,
                    under,
                    &MotionEvent {
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
                        let window = window.clone();
                        self.focus_window(Some(&window));
                        self.renderer.elements().for_each(|window| {
                            window.toplevel().map(|t| t.send_pending_configure());
                        });
                    }
                    ButtonState::Released => {}
                }

                let serial = SERIAL_COUNTER.next_serial();
                pointer.button(
                    self,
                    &ButtonEvent {
                        button: event.button_code(),
                        state: button_state,
                        serial,
                        time: event.time_msec(),
                    },
                );
            }
            _ => {}
        }

        Some(())
    }

    pub fn focus_window(&mut self, window: Option<&Window>) {
        if let Some(window) = window {
            self.renderer.raise_element(&window, true);
        }
        self.seat.get_keyboard().map(|keyboard| {
            let serial = SERIAL_COUNTER.next_serial();
            keyboard.set_focus(
                self,
                window.and_then(Window::wl_surface).map(Cow::into_owned),
                serial,
            );
        });
    }
}
