use crate::grabs::{MoveGrab, ResizeGrab, ResizeState};
use crate::PocoWM;
use smithay::delegate_xdg_shell;
use smithay::desktop::{
    find_popup_root_surface, get_popup_toplevel_coords, PopupKind, PopupManager, Space, Window,
};
use smithay::input::pointer::{Focus, GrabStartData};
use smithay::input::Seat;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::{self, ResizeEdge};
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::Resource as _;
use smithay::utils::{Rectangle, Serial};
use smithay::wayland::compositor::with_states;
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
    XdgToplevelSurfaceData,
};

impl XdgShellHandler for PocoWM {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new_wayland_window(surface);
        self.space.map_element(window, (0, 0), false);
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        self.unconstrain_popup(&surface);
        let _ = self.popups.track_popup(PopupKind::Xdg(surface));
    }

    fn grab(&mut self, _surface: PopupSurface, _seatt: WlSeat, _serial: Serial) {
        // TODO: Implement popups grab
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        surface.with_pending_state(|state| {
            let geometry = positioner.get_geometry();
            state.geometry = geometry;
            state.positioner = positioner;
        });
        self.unconstrain_popup(&surface);
        surface.send_repositioned(token);
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: WlSeat, serial: Serial) {
        let Some(seat) = Seat::from_resource(&seat) else {
            return;
        };

        let wl_surface = surface.wl_surface();

        if let Some(start_data) = check_grab(&seat, &wl_surface, serial) {
            let Some(pointer) = seat.get_pointer() else {
                return;
            };
            let Some(window) = self
                .space
                .elements()
                .find(|w| w.toplevel().is_some_and(|t| t.wl_surface() == wl_surface))
            else {
                return;
            };
            let Some(initial_window_location) = self.space.element_location(window) else {
                return;
            };

            let grab = MoveGrab {
                start_data,
                window: window.clone(),
                initial_window_location,
            };

            pointer.set_grab(self, grab, serial, Focus::Clear)
        }
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: WlSeat,
        serial: Serial,
        edges: ResizeEdge,
    ) {
        let Some(seat) = Seat::from_resource(&seat) else {
            return;
        };
        let wl_surface = surface.wl_surface();
        let Some(start_data) = check_grab(&seat, &wl_surface, serial) else {
            return;
        };
        let Some(pointer) = seat.get_pointer() else {
            return;
        };
        let Some(window) = self
            .space
            .elements()
            .find(|w| w.toplevel().is_some_and(|t| t.wl_surface() == wl_surface))
        else {
            return;
        };
        let Some(initial_window_location) = self.space.element_location(window) else {
            return;
        };
        let initial_window_size = window.geometry().size;
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Resizing);
        });
        surface.send_pending_configure();

        let initial_window_rect =
            Rectangle::from_loc_and_size(initial_window_location, initial_window_size);

        ResizeState::with(surface.wl_surface(), |state| {
            *state = ResizeState::Resizing {
                edges: edges.into(),
                initial_rect: initial_window_rect,
            };
        });

        let grab = ResizeGrab {
            start_data,
            window: window.clone(),
            initial_rect: initial_window_rect,
            last_window_size: initial_window_rect.size,
            edges: edges.into(),
        };

        pointer.set_grab(self, grab, serial, Focus::Clear)
    }
}

fn check_grab(
    seat: &Seat<PocoWM>,
    surface: &WlSurface,
    serial: Serial,
) -> Option<GrabStartData<PocoWM>> {
    let pointer = seat.get_pointer()?;
    if !pointer.has_grab(serial) {
        return None;
    }
    let start_data = pointer.grab_start_data()?;
    let (focus, _) = start_data.focus.as_ref()?;
    if !focus.id().same_client_as(&surface.id()) {
        return None;
    }
    Some(start_data)
}

impl PocoWM {
    fn unconstrain_popup(&mut self, surface: &PopupSurface) {
        let Ok(root) = find_popup_root_surface(&PopupKind::Xdg(surface.clone())) else {
            return;
        };
        let Some(window) = self
            .space
            .elements()
            .find(|w| w.toplevel().map_or(false, |t| t.wl_surface() == &root))
        else {
            return;
        };

        let Some(output) = self.space.outputs().next() else {
            return;
        };
        let Some(output_geometry) = self.space.output_geometry(output) else {
            return;
        };
        let Some(window_geometry) = self.space.element_geometry(window) else {
            return;
        };

        let mut target_geometry = output_geometry.clone();
        target_geometry.loc -= get_popup_toplevel_coords(&PopupKind::Xdg(surface.clone()));
        target_geometry.loc -= window_geometry.loc;
        surface.with_pending_state(|state| {
            state.geometry = state.positioner.get_unconstrained_geometry(target_geometry);
        })
    }
}

pub(super) fn handle_commit(popups: &mut PopupManager, space: &Space<Window>, surface: &WlSurface) {
    space
        .elements()
        .find(|window| {
            window
                .toplevel()
                .map_or(false, |t| t.wl_surface() == surface)
        })
        .map(|window| {
            with_states(surface, |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .and_then(|data| data.lock().ok())
                    .map_or(false, |data| !data.initial_configure_sent)
            })
            .then(|| {
                window.toplevel().map(|t| t.send_configure());
            });
        });

    popups.commit(surface);
    popups.find_popup(surface).map(|popup| match popup {
        PopupKind::Xdg(xdg) => {
            if !xdg.is_initial_configure_sent() {
                xdg.send_configure().unwrap();
            }
        }
        PopupKind::InputMethod(_) => {}
    });
}

delegate_xdg_shell!(PocoWM);
