use anyhow::{Context as _, Result};
use smithay::desktop::{PopupManager, Space, Window, WindowSurfaceType};
use smithay::input::{Seat, SeatState};
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{self, EventLoop, LoopSignal};
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Display, DisplayHandle};
use smithay::utils::{Logical, Point};
use smithay::wayland::compositor::{CompositorClientState, CompositorState};
use smithay::wayland::seat::WaylandFocus;
use smithay::wayland::selection::data_device::DataDeviceState;
use smithay::wayland::shell::xdg::XdgShellState;
use smithay::wayland::shm::ShmState;
use smithay::wayland::socket::ListeningSocketSource;
use std::cell::RefCell;
use std::ffi::OsString;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug)]
pub struct PocoWM {
    pub(crate) start_time: Instant,
    pub(crate) event_loop: Rc<RefCell<EventLoop<'static, Self>>>,
    pub(crate) display: DisplayHandle,
    pub(crate) seat: Seat<Self>,
    pub(crate) space: Space<Window>,
    pub(crate) loop_signal: LoopSignal,
    pub(crate) socket_name: OsString,
    pub(crate) popups: PopupManager,

    pub(crate) seat_state: SeatState<Self>,
    pub(crate) data_device_state: DataDeviceState,
    pub(crate) compositor_state: CompositorState,
    pub(crate) xdg_shell_state: XdgShellState,
    pub(crate) shm_state: ShmState,
}

impl PocoWM {
    pub fn new() -> Result<Self> {
        let start_time = Instant::now();
        let display = Display::<Self>::new().context("Failed to init display")?;
        let dh = display.handle();
        let event_loop = EventLoop::<Self>::try_new().context("Failed to init event loop")?;
        let loop_signal = event_loop.get_signal();
        let socket = ListeningSocketSource::new_auto().context("Failed to init socket")?;
        let mut seat_state = SeatState::<Self>::new();
        let data_device_state = DataDeviceState::new::<Self>(&dh);
        let space = Space::default();
        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let popups = PopupManager::default();

        let mut seat: Seat<Self> = seat_state.new_wl_seat(&dh, "winit");
        let socket_name = socket.socket_name().to_owned();

        event_loop
            .handle()
            .insert_source(socket, move |client, _, state| {
                let result = state
                    .display
                    .insert_client(client, Arc::new(ClientState::default()))
                    .context("Failed to init client");
                if let Err(err) = result {
                    eprintln!("{:?}", err);
                }
            })
            .context("Failed to init wayland event source")?;

        event_loop
            .handle()
            .insert_source(
                Generic::new(display, calloop::Interest::READ, calloop::Mode::Level),
                |_, display, state| {
                    let result = unsafe { display.get_mut().dispatch_clients(state) };
                    if let Err(err) = result {
                        eprintln!("{:?}", err);
                    }
                    Ok(calloop::PostAction::Continue)
                },
            )
            .context("Failed to init display event source")?;

        seat.add_keyboard(Default::default(), 200, 25)
            .context("Failed to init keyboard")?;

        seat.add_pointer();

        Ok(Self {
            event_loop: Rc::new(RefCell::new(event_loop)),
            start_time,
            display: dh,
            seat,
            space,
            loop_signal,
            socket_name,
            popups,

            compositor_state,
            data_device_state,
            seat_state,
            shm_state,
            xdg_shell_state,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        println!("Listening on {}", self.socket_name.to_string_lossy());

        self.event_loop
            .clone()
            .borrow_mut()
            .run(None, self, |_| {})
            .context("Failed to run event loop")?;

        Ok(())
    }

    pub(crate) fn surface_under(
        &self,
        pos: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space
            .element_under(pos)
            .and_then(|(window, location)| {
                window
                    .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                    .map(|(s, p)| (s, (p + location).to_f64()))
            })
    }

    pub(crate) fn get_window(&self, wl_surface: &WlSurface) -> Option<&Window> {
        self.space.elements().find(|window| {
            window
                .wl_surface()
                .is_some_and(|s| s.as_ref() == wl_surface)
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct ClientState(pub(crate) CompositorClientState);

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
