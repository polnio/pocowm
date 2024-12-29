use crate::grabs::resize_grab::ResizeEdge;
use crate::grabs::{MoveGrab, ResizeGrab, ResizeState};
use crate::window::Window;
use crate::PocoWM;
use smithay::delegate_xdg_shell;
use smithay::desktop::{find_popup_root_surface, get_popup_toplevel_coords, PopupKind};
use smithay::input::pointer::{Focus, GrabStartData};
use smithay::input::Seat;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::{self};
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::Resource as _;
use smithay::utils::{Rectangle, Serial};
use smithay::wayland::compositor::with_states;
use smithay::wayland::seat::WaylandFocus;
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
    XdgToplevelSurfaceData,
};

impl XdgShellHandler for PocoWM {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::from_surface(surface);
        let mut positions = self
            .seat
            .get_keyboard()
            .and_then(|k| k.current_focus())
            .and_then(|w| self.layout.get_window_positions(&w));
        positions
            .as_mut()
            .and_then(|p| p.last_mut())
            .map(|p| *p += 1);
        self.layout.add_window(window.clone(), positions.as_deref());
        self.renderer
            .map_element(window.clone().into(), (0, 0), false);
        self.renderer.render(&self.layout);
        self.focus_window(Some(&window));
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let window = self.layout.get_window_from_surface(surface.wl_surface());
        window.map(|w| self.renderer.unmap_elem(&w.clone().into()));
        let mut positions = window.and_then(|w| self.layout.get_window_positions(w));
        self.layout.remove_window(positions.as_deref());
        self.renderer.render(&self.layout);
        positions
            .as_mut()
            .and_then(|p| p.last_mut())
            .map(|p| *p = p.saturating_sub(1));
        positions
            .and_then(|p| self.layout.get_window(&p))
            .cloned()
            .map(|w| self.focus_window(Some(&w)));
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

        self.xdg_move_request(&surface, &seat, serial);
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: WlSeat,
        serial: Serial,
        edges: xdg_toplevel::ResizeEdge,
    ) {
        let Some(seat) = Seat::from_resource(&seat) else {
            return;
        };
        self.xdg_resize_request(&surface, &seat, serial, edges.into());
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
    let focused_surface = focus.wl_surface()?;
    if !focused_surface.id().same_client_as(&surface.id()) {
        return None;
    }
    Some(start_data)
}

impl PocoWM {
    fn unconstrain_popup(&mut self, surface: &PopupSurface) {
        let Ok(root) = find_popup_root_surface(&PopupKind::Xdg(surface.clone())) else {
            return;
        };
        let Some(window) = self.layout.get_window_from_surface(&root) else {
            return;
        };

        let Some(output) = self.renderer.outputs().next() else {
            return;
        };
        let Some(output_geometry) = self.renderer.output_geometry(output) else {
            return;
        };
        let Some(window_geometry) = self.renderer.element_geometry(&window.clone().into()) else {
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

impl PocoWM {
    pub fn xdg_move_request(
        &mut self,
        surface: &ToplevelSurface,
        seat: &Seat<PocoWM>,
        serial: Serial,
    ) {
        let Some(pointer) = seat.get_pointer() else {
            return;
        };
        if !pointer.has_grab(serial) {
            return;
        }

        let wl_surface = surface.wl_surface();
        // let Some(start_data) = check_grab(&seat, &wl_surface, serial) else {
        //     return;
        // };
        let Some(start_data) = pointer.grab_start_data() else {
            return;
        };
        if !start_data
            .focus
            .as_ref()
            .is_some_and(|f| f.0.same_client_as(&surface.wl_surface().id()))
        {
            return;
        }
        let Some(window) = self.layout.get_window_from_surface(&wl_surface) else {
            return;
        };
        if !window.state().is_floating() {
            return;
        }
        let Some(initial_window_location) = self.renderer.element_location(&window.clone().into())
        else {
            return;
        };

        let grab = MoveGrab {
            start_data,
            window: window.clone(),
            initial_window_location,
        };

        pointer.set_grab(self, grab, serial, Focus::Clear)
    }

    pub fn xdg_resize_request(
        &mut self,
        surface: &ToplevelSurface,
        seat: &Seat<PocoWM>,
        serial: Serial,
        edges: ResizeEdge,
    ) {
        let wl_surface = surface.wl_surface();
        let Some(pointer) = seat.get_pointer() else {
            return;
        };
        let Some(window) = self.layout.get_window_from_surface(wl_surface) else {
            return;
        };
        if !window.state().is_floating() {
            return;
        }
        let Some(start_data) = check_grab(&seat, wl_surface, serial) else {
            return;
        };
        let Some(initial_window_location) = self.renderer.element_location(&window.clone().into())
        else {
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
                edges,
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

pub(super) fn handle_commit(state: &mut PocoWM, surface: &WlSurface) {
    state.layout.get_window_from_surface(surface).map(|window| {
        with_states(surface, |states| {
            states
                .data_map
                .get::<XdgToplevelSurfaceData>()
                .and_then(|data| data.lock().ok())
                .is_some_and(|data| !data.initial_configure_sent)
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
