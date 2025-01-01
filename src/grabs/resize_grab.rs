use crate::utils::Edge;
use crate::window::Window;
use crate::PocoWM;
use smithay::input::pointer::{
    AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
    GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
    GestureSwipeUpdateEvent, GrabStartData, MotionEvent, PointerGrab, PointerInnerHandle,
    RelativeMotionEvent,
};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point, Rectangle, Size};
use smithay::wayland::compositor::with_states;
use smithay::wayland::seat::WaylandFocus as _;
use smithay::wayland::shell::xdg::SurfaceCachedState;
use std::cell::RefCell;
use std::num::NonZeroI32;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ResizeState {
    #[default]
    Idle,
    Resizing {
        edges: Edge,
        initial_rect: Rectangle<i32, Logical>,
    },
    WaitingForLastCommit {
        edges: Edge,
        initial_rect: Rectangle<i32, Logical>,
    },
}

impl ResizeState {
    pub fn with<T>(surface: &WlSurface, f: impl FnOnce(&mut Self) -> T) -> Option<T> {
        with_states(surface, |states| {
            states.data_map.insert_if_missing(RefCell::<Self>::default);
            let state = states.data_map.get::<RefCell<Self>>()?;
            Some(f(&mut state.borrow_mut()))
        })
    }

    fn commit(&mut self) -> Option<(Edge, Rectangle<i32, Logical>)> {
        match *self {
            Self::Idle => None,
            Self::Resizing {
                edges,
                initial_rect,
            } => Some((edges, initial_rect)),
            Self::WaitingForLastCommit {
                edges,
                initial_rect,
            } => {
                *self = Self::Idle;
                Some((edges, initial_rect))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResizeGrab {
    pub start_data: GrabStartData<PocoWM>,
    pub window: Window,
    pub edges: Edge,
    pub initial_rect: Rectangle<i32, Logical>,
    pub last_window_size: Size<i32, Logical>,
}

impl PointerGrab<PocoWM> for ResizeGrab {
    fn motion(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        _focus: Option<(Window, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);
        let mut delta = event.location - self.start_data.location;
        let mut new_window_width = self.initial_rect.size.w;
        let mut new_window_height = self.initial_rect.size.h;
        if self.edges.intersects(Edge::LEFT) {
            delta.x = -delta.x;
        }
        if self.edges.intersects(Edge::LEFT | Edge::RIGHT) {
            new_window_width = (self.initial_rect.size.w as f64 + delta.x) as i32;
        }
        if self.edges.intersects(Edge::TOP) {
            delta.y = -delta.y;
        }
        if self.edges.intersects(Edge::TOP | Edge::BOTTOM) {
            new_window_height = (self.initial_rect.size.h as f64 + delta.y) as i32;
        }
        let Some(wl_surface) = self.window.wl_surface() else {
            return;
        };
        let (min_size, max_size) = with_states(&wl_surface, |states| {
            let guard = &mut states.cached_state.get::<SurfaceCachedState>();
            let data = guard.current();
            (data.min_size, data.max_size)
        });
        let min_width = min_size.w.max(1);
        let min_height = min_size.h.max(1);
        let max_width = NonZeroI32::new(max_size.w)
            .map(NonZeroI32::get)
            .unwrap_or(i32::MAX);
        let max_height = NonZeroI32::new(max_size.h)
            .map(NonZeroI32::get)
            .unwrap_or(i32::MAX);
        self.last_window_size = Size::from((
            new_window_width.max(min_width).min(max_width),
            new_window_height.max(min_height).min(max_height),
        ));

        let Some(xdg) = self.window.toplevel() else {
            return;
        };
        xdg.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Resizing);
            state.size = Some(self.last_window_size);
        });
        xdg.send_pending_configure();
    }

    fn relative_motion(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        focus: Option<(Window, Point<f64, Logical>)>,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event);
    }

    fn button(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);
        const BTN_LEFT: u32 = 0x110;
        if !handle.current_pressed().contains(&BTN_LEFT) {
            handle.unset_grab(self, data, event.serial, event.time, true);
        }
    }

    fn axis(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        details: AxisFrame,
    ) {
        handle.axis(data, details);
    }

    fn frame(&mut self, data: &mut PocoWM, handle: &mut PointerInnerHandle<'_, PocoWM>) {
        handle.frame(data);
    }

    fn gesture_swipe_begin(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event);
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event);
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event);
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event);
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event);
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event);
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event);
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut PocoWM,
        handle: &mut PointerInnerHandle<'_, PocoWM>,
        event: &GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event);
    }

    fn start_data(&self) -> &GrabStartData<PocoWM> {
        &self.start_data
    }

    fn unset(&mut self, _data: &mut PocoWM) {
        let Some(xdg) = self.window.toplevel() else {
            return;
        };
        xdg.with_pending_state(|state| {
            state.states.unset(xdg_toplevel::State::Resizing);
            state.size = Some(self.last_window_size);
        });
        xdg.send_pending_configure();
        if self.window.state().is_floating() {
            self.window.floating_rect_mut().size = self.last_window_size;
        }
        ResizeState::with(xdg.wl_surface(), |state| {
            *state = ResizeState::WaitingForLastCommit {
                edges: self.edges,
                initial_rect: self.initial_rect,
            };
        });
    }
}

/// Should be called on `WlSurface::commit`
pub(crate) fn handle_commit(state: &mut PocoWM, surface: &WlSurface) -> Option<()> {
    let window = state.layout.get_mut_window_from_surface(surface)?;
    if !window.state().is_floating() {
        return None;
    }

    let mut window_loc = state.renderer.element_location(window)?;
    let geometry = window.geometry();

    let new_loc: Point<Option<i32>, Logical> = ResizeState::with(surface, |state| {
        state
            .commit()
            .and_then(|(edges, initial_rect)| {
                edges.intersects(Edge::TOP | Edge::LEFT).then(|| {
                    let new_x = edges
                        .intersects(Edge::LEFT)
                        .then_some(initial_rect.loc.x + (initial_rect.size.w - geometry.size.w));
                    let new_y = edges
                        .intersects(Edge::TOP)
                        .then_some(initial_rect.loc.y + (initial_rect.size.h - geometry.size.h));
                    (new_x, new_y).into()
                })
            })
            .unwrap_or_default()
    })
    .unwrap_or_default();

    if let Some(new_x) = new_loc.x {
        window_loc.x = new_x;
    }
    if let Some(new_y) = new_loc.y {
        window_loc.y = new_y;
    }

    if new_loc.x.is_some() || new_loc.y.is_some() {
        window.floating_rect_mut().loc = window_loc;
        state
            .renderer
            .map_element(window.clone(), window_loc, false);
    }

    Some(())
}
