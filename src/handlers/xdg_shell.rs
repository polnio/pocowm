use crate::grabs::{MoveGrab, ResizeGrab, ResizeState};
use crate::layout::{Id, Layout, LayoutElement};
use crate::utils::Edge;
use crate::window::{Window, WindowState};
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

fn get_next_focus_id(layout: &Layout, id: Id, before: bool) -> Option<Id> {
    let element = layout.get_element(id)?;
    match element {
        LayoutElement::Window(w) if !before => Some(w.id),
        LayoutElement::Window(w) => get_next_focus_id(layout, w.parent, true),
        LayoutElement::SubLayout(sl) => {
            let index = match (sl.last_focused, before) {
                (0, _) => 1,
                (i, true) => i - 1,
                (i, false) => i,
            };
            if let Some(el) = sl.children.get(index) {
                get_next_focus_id(layout, *el, false)
            } else if let Some(p) = sl.parent {
                get_next_focus_id(layout, p, true)
            } else {
                None
            }
        }
    }
}

impl XdgShellHandler for PocoWM {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::from_surface(surface);
        let output_geo = self
            .renderer
            .outputs()
            .next()
            .and_then(|o| self.renderer.output_geometry(o))
            .unwrap_or_default();
        *window.floating_rect_mut() = Rectangle::new(
            (output_geo.size.w / 4, output_geo.size.h / 4).into(),
            (output_geo.size.w / 2, output_geo.size.h / 2).into(),
        );
        let id = self
            .seat
            .get_keyboard()
            .and_then(|k| k.current_focus())
            .and_then(|w| self.layout.get_window_id(&w))
            .unwrap_or_default();
        let Some(new_id) = self.layout.insert_window(id, window.clone()) else {
            return;
        };
        // self.renderer.map_element(window.clone(), (0, 0), false);
        self.renderer.render(&self.layout);
        self.focus_window(Some(new_id));
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let window = self
            .layout
            .get_window_from_surface(surface.wl_surface())
            .unwrap();
        self.renderer.unmap_elem(window);
        let id = self.layout.get_window_id(window).unwrap();
        let focus_id = get_next_focus_id(&self.layout, id, true);
        self.layout.remove_element(id);
        self.renderer.render(&self.layout);

        self.focus_window(focus_id);
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        self.unconstrain_popup(&surface);
        let _ = self.popups.track_popup(PopupKind::Xdg(surface));
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
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

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        self.xdg_maximize_request(&surface);
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        self.xdg_unmaximize_request(&surface);
    }

    fn minimize_request(&mut self, surface: ToplevelSurface) {
        self.xdg_minimize_request(&surface);
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
        let Some(window_geometry) = self.renderer.element_geometry(window) else {
            return;
        };

        let mut target_geometry = output_geometry.clone();
        target_geometry.loc -= get_popup_toplevel_coords(&PopupKind::Xdg(surface.clone()));
        target_geometry.loc -= window_geometry.loc;
        surface.with_pending_state(|state| {
            state.geometry = state.positioner.get_unconstrained_geometry(target_geometry);
        })
    }

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
        // if *window.state() != WindowState::FLOATING {
        //     return;
        // }
        let Some(initial_window_location) = self.renderer.element_location(window) else {
            return;
        };

        let grab = MoveGrab {
            start_data,
            window: window.clone(),
            initial_window_location,
            new_location: initial_window_location,
            pointer_location: pointer.current_location(),
        };

        pointer.set_grab(self, grab, serial, Focus::Clear)
    }

    pub fn xdg_resize_request(
        &mut self,
        surface: &ToplevelSurface,
        seat: &Seat<PocoWM>,
        serial: Serial,
        edges: Edge,
    ) {
        let wl_surface = surface.wl_surface();
        let Some(pointer) = seat.get_pointer() else {
            return;
        };
        let Some(window) = self.layout.get_window_from_surface(wl_surface) else {
            return;
        };
        if *window.state() != WindowState::FLOATING {
            return;
        }
        let Some(start_data) = check_grab(&seat, wl_surface, serial) else {
            return;
        };
        let Some(initial_window_location) = self.renderer.element_location(window) else {
            return;
        };
        let initial_window_size = window.geometry().size;
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Resizing);
        });
        surface.send_pending_configure();

        let initial_window_rect = Rectangle::new(initial_window_location, initial_window_size);

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

    pub fn xdg_maximize_request(&mut self, surface: &ToplevelSurface) {
        let Some(window) = self.layout.get_window_from_surface(surface.wl_surface()) else {
            return;
        };
        window.state_mut().insert(WindowState::MAXIMIZED);
        self.renderer.render(&self.layout);
    }

    pub fn xdg_unmaximize_request(&mut self, surface: &ToplevelSurface) {
        let Some(window) = self.layout.get_window_from_surface(surface.wl_surface()) else {
            return;
        };
        window.state_mut().remove(WindowState::MAXIMIZED);
        self.renderer.render(&self.layout);
    }

    pub fn xdg_minimize_request(&mut self, surface: &ToplevelSurface) {
        let Some(window) = self.layout.get_window_from_surface(surface.wl_surface()) else {
            return;
        };
        window.state_mut().insert(WindowState::MINIMIZED);
        self.renderer.render(&self.layout);
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
