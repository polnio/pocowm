use crate::grabs::{MoveGrab, ResizeGrab, ResizeState};
use crate::PocoWM;
use smithay::delegate_xdg_shell;
use smithay::desktop::{find_popup_root_surface, get_popup_toplevel_coords, PopupKind, Window};
use smithay::input::pointer::{Focus, GrabStartData};
use smithay::input::Seat;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::{self, ResizeEdge};
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::Resource as _;
use smithay::utils::{Logical, Rectangle, Serial};
use smithay::wayland::compositor::with_states;
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
    XdgToplevelSurfaceData,
};

impl PocoWM {
    fn tile_windows(&mut self) -> Option<()> {
        let output = self.space.outputs().next()?;
        let output_geometry = self.space.output_geometry(output)?;
        let elements_count = self.space.elements().count();

        self.space
            .elements()
            .enumerate()
            .map(|(i, window)| {
                let mut x = 0;
                let mut y = 0;
                let mut width = output_geometry.size.w;
                let mut height = output_geometry.size.h;
                if elements_count > 1 {
                    width /= 2;
                }
                if i > 0 {
                    height /= elements_count as i32 - 1;
                    x += width;
                    y += height * (i as i32 - 1);
                }

                (
                    window.clone(),
                    Rectangle::<i32, Logical>::from_loc_and_size((x, y), (width, height)),
                )
            })
            .collect::<Vec<_>>()
            .into_iter()
            .try_for_each(|(window, rect)| {
                window.toplevel()?.with_pending_state(|state| {
                    state.size = Some(rect.size);
                });
                window.toplevel()?.send_configure();
                self.space.map_element(window.clone(), rect.loc, false);

                Some(())
            });

        Some(())
    }
}

impl XdgShellHandler for PocoWM {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new_wayland_window(surface);
        self.space.map_element(window.clone(), (0, 0), false);
        self.focus_window(Some(&window));
        self.tile_windows();
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let Some(window) = self.get_window(surface.wl_surface()).cloned() else {
            return;
        };
        self.space.unmap_elem(&window);
        self.tile_windows();
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
            let Some(window) = self.get_window(wl_surface) else {
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
        let Some(window) = self.get_window(wl_surface) else {
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
        let Some(window) = self.get_window(&root) else {
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

pub(super) fn handle_commit(state: &mut PocoWM, surface: &WlSurface) {
    state.get_window(surface).map(|window| {
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

    state.popups.commit(surface);
    state.popups.find_popup(surface).map(|popup| match popup {
        PopupKind::Xdg(xdg) => {
            if !xdg.is_initial_configure_sent() {
                let _ = xdg.send_configure();
            }
        }
        PopupKind::InputMethod(_) => {}
    });
}

delegate_xdg_shell!(PocoWM);
