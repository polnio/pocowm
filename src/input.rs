use smithay::backend::input::{
    AbsolutePositionEvent as _, ButtonState, Event as _, InputBackend, InputEvent,
    KeyboardKeyEvent as _, PointerButtonEvent as _,
};
use smithay::input::keyboard::FilterResult;
use smithay::input::pointer::{ButtonEvent, MotionEvent};
use smithay::utils::SERIAL_COUNTER;

use crate::PocoWM;

impl PocoWM {
    pub(crate) fn handle_input<B: InputBackend>(&mut self, event: InputEvent<B>) -> Option<()> {
        match event {
            InputEvent::Keyboard { event } => {
                let serial = SERIAL_COUNTER.next_serial();
                let time = smithay::backend::input::Event::time_msec(&event);
                self.seat.get_keyboard().unwrap().input::<(), _>(
                    self,
                    event.key_code(),
                    event.state(),
                    serial,
                    time,
                    |_, _, _| FilterResult::Forward,
                );
            }
            InputEvent::PointerMotion { .. } => {}
            InputEvent::PointerMotionAbsolute { event, .. } => {
                let output = self.space.outputs().next()?;
                let output_geometry = self.space.output_geometry(output)?;
                let pos =
                    event.position_transformed(output_geometry.size) + output_geometry.loc.to_f64();
                let serial = SERIAL_COUNTER.next_serial();
                let pointer = self.seat.get_pointer()?;

                let under = self.surface_under(pos);
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

                /* if let Some(surface) =
                    self.xdg_shell_state.toplevel_surfaces().iter().next()
                {
                    let surface = surface.wl_surface();
                    self.seat.get_keyboard().unwrap().set_focus(
                        self,
                        Some(surface.clone()),
                        0.into(),
                    );
                } */
            }
            InputEvent::PointerButton { event, .. } => {
                let pointer = self.seat.get_pointer()?;
                let keyboard = self.seat.get_keyboard()?;
                let serial = SERIAL_COUNTER.next_serial();
                let button_state = event.state();
                match button_state {
                    ButtonState::Pressed if pointer.is_grabbed() => {
                        self.space.elements().for_each(|window| {
                            window.set_activated(false);
                            window.toplevel().map(|t| t.send_pending_configure());
                        });
                        keyboard.set_focus(self, None, serial);
                    }
                    ButtonState::Pressed => {
                        let (window, _location) =
                            self.space.element_under(pointer.current_location())?;
                        let window = window.clone();
                        self.space.raise_element(&window, true);
                        keyboard.set_focus(
                            self,
                            window.toplevel().map(|t| t.wl_surface().clone()),
                            serial,
                        );
                        self.space.elements().for_each(|window| {
                            window.toplevel().map(|t| t.send_pending_configure());
                        });
                    }
                    ButtonState::Released => {}
                }

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
}
