use crate::layout::{LayoutElement, LayoutType};
use crate::window::Window;
use crate::PocoWM;
use bitflags::bitflags;
use smithay::backend::input::{
    AbsolutePositionEvent as _, ButtonState, Event, InputBackend, InputEvent, KeyState,
    KeyboardKeyEvent as _, PointerButtonEvent as _,
};
use smithay::input::keyboard::{FilterResult, Keysym, ModifiersState};
use smithay::input::pointer::{ButtonEvent, MotionEvent};
use smithay::utils::SERIAL_COUNTER;
use smithay::wayland::seat::WaylandFocus as _;
use std::borrow::Cow;

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
                            let syms = key.raw_syms();
                            if syms.contains(&Keysym::Return) {
                                let _ = std::process::Command::new("kitty")
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .spawn();
                                return FilterResult::Intercept(());
                            }
                            if syms.contains(&Keysym::e) {
                                keyboard
                                    .current_focus()
                                    .and_then(|s| state.layout.get_window_from_surface(&s))
                                    .and_then(|w| w.state().is_tiled().then_some(w))
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
                                return FilterResult::Intercept(());
                            }

                            if syms.contains(&Keysym::b) {
                                state.switch_to_layout(LayoutType::Vertical);
                                return FilterResult::Intercept(());
                            }
                            if syms.contains(&Keysym::n) {
                                state.switch_to_layout(LayoutType::Horizontal);
                                return FilterResult::Intercept(());
                            }
                            if syms.contains(&Keysym::f) {
                                state.toggle_floating();
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

                let pointed_window = self
                    .renderer
                    .element_under(pos)
                    .map(|(w, _)| Window::from(w.clone()));
                let focused_window = self
                    .seat
                    .get_keyboard()
                    .and_then(|k| k.current_focus())
                    .and_then(|s| self.layout.get_window_from_surface(&s))
                    .cloned();
                Option::zip(pointed_window, focused_window).map(
                    |(pointed_window, focused_window)| {
                        if pointed_window != focused_window {
                            self.focus_window(Some(&pointed_window));
                        }
                    },
                );

                pointer.motion(
                    self,
                    self.renderer.surface_under(pos),
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
                        let window = window.clone().into();
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
            self.layout.iter_mut_windows().for_each(|w| {
                w.set_activated(false);
            });
            window.set_activated(true);
        }
        self.seat.get_keyboard().map(|keyboard| {
            let serial = SERIAL_COUNTER.next_serial();
            keyboard.set_focus(
                self,
                window.and_then(|w| w.wl_surface()).map(Cow::into_owned),
                serial,
            );
        });
    }
}
