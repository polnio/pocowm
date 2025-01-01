use smithay::delegate_xdg_decoration;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
use smithay::wayland::shell::xdg::decoration::XdgDecorationHandler;
use smithay::wayland::shell::xdg::ToplevelSurface;
use crate::PocoWM;

impl XdgDecorationHandler for PocoWM {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        XdgDecorationHandler::request_mode(self, toplevel, Mode::ServerSide);
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        XdgDecorationHandler::request_mode(self, toplevel, Mode::ServerSide);
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, mode: Mode) {
        if let Some(window) = self.layout.get_window_from_surface(toplevel.wl_surface()) {
            match mode {
                Mode::ServerSide => window.add_decorations(),
                Mode::ClientSide => window.remove_decorations(),
                _ => {}
            };
        }
        self.renderer.render(&self.layout);
    }
}

delegate_xdg_decoration!(PocoWM);
