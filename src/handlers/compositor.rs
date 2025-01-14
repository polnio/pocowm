use std::borrow::Cow;

use crate::state::ClientState;
use crate::PocoWM;
use smithay::backend::renderer::utils::on_commit_buffer_handler;
use smithay::delegate_compositor;
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::Client;
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::compositor::{
    get_parent, is_sync_subsurface, CompositorClientState, CompositorHandler, CompositorState,
};

impl BufferHandler for PocoWM {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl CompositorHandler for PocoWM {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().0
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        if !is_sync_subsurface(surface) {
            let mut root = Cow::Borrowed(surface);
            while let Some(parent) = get_parent(&root) {
                root = Cow::Owned(parent);
            }
            self.layout
                .get_window_from_surface(&root)
                .map(|w| w.on_commit());
        }
        super::xdg_shell::handle_commit(self, surface);
        crate::grabs::resize_grab::handle_commit(self, surface);
    }
}
delegate_compositor!(PocoWM);
