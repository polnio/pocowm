use crate::renderer::WindowElements;
use crate::PocoWM;
use smithay::delegate_seat;
use smithay::input::pointer::CursorImageStatus;
use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::reexports::wayland_server::Resource as _;
use smithay::wayland::seat::WaylandFocus as _;
use smithay::wayland::selection::data_device::set_data_device_focus;

impl SeatHandler for PocoWM {
    // type PointerFocus = WlSurface;
    // type KeyboardFocus = WlSurface;
    // type TouchFocus = WlSurface;

    type KeyboardFocus = WindowElements;
    type PointerFocus = WindowElements;
    type TouchFocus = WindowElements;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: CursorImageStatus) {}

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&Self::KeyboardFocus>) {
        let client = focused
            .and_then(|f| f.wl_surface())
            .and_then(|f| self.display.get_client(f.id()).ok());
        set_data_device_focus(&self.display, seat, client);
    }
}
delegate_seat!(PocoWM);
