use super::decorations::{self, BUTTON_GAP, DECORATIONS_SIZE};
use super::DecorationsElements;
use crate::grabs::resize_grab::ResizeEdge;
use crate::window::Window;
use crate::PocoWM;
use derive_more::{Deref, DerefMut};
use smithay::backend::input::KeyState;
use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::element::AsRenderElements;
use smithay::backend::renderer::{ImportAll, ImportMem, Renderer, Texture};
use smithay::desktop::space::SpaceElement;
use smithay::input::keyboard::{self, KeyboardTarget, KeysymHandle};
use smithay::input::pointer::{self, PointerTarget};
use smithay::input::touch::{self, TouchTarget};
use smithay::input::Seat;
use smithay::output::Output;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::render_elements;
use smithay::utils::{IsAlive, Logical, Physical, Point, Rectangle, Scale, Serial};
use smithay::wayland::seat::WaylandFocus;
use std::borrow::Cow;
use std::cell::RefCell;
use std::sync::{Arc, RwLock};

const RESIZE_GRAB_SIZE: u32 = BUTTON_GAP;

#[derive(Debug, Clone, Default, PartialEq)]
struct TargetState {
    pub pointer_location: Option<Point<f64, Logical>>,
    pub touch_location: Option<Point<f64, Logical>>,
}

#[derive(Debug, Clone)]
pub struct WindowElements {
    window: Arc<Window>,
    decorations: RefCell<DecorationsElements>,
    target_state: Arc<RwLock<TargetState>>,
}

impl WindowElements {
    pub fn new(window: Arc<Window>) -> Self {
        Self {
            window,
            decorations: RefCell::new(DecorationsElements::new()),
            target_state: Arc::new(RwLock::new(TargetState::default())),
        }
    }
    pub fn inner(&self) -> &Window {
        &self.window
    }
}

impl PartialEq for WindowElements {
    fn eq(&self, other: &Self) -> bool {
        self.window == other.window
    }
}
impl Eq for WindowElements {}

impl Deref for WindowElements {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

// impl DerefMut for WindowElements {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.window
//     }
// }

// impl From<Window> for WindowElements {
//     fn from(window: Window) -> Self {
//         Self {
//             window,
//             decorations: RefCell::new(DecorationsElements::new()),
//             target_state: Arc::new(RwLock::new(TargetState::default())),
//         }
//     }
// }

render_elements! {
    pub WindowRenderElement<R> where R: ImportAll + ImportMem;
    Window=WaylandSurfaceRenderElement<R>,
    Decorations=SolidColorRenderElement,
}

impl IsAlive for WindowElements {
    fn alive(&self) -> bool {
        self.window.alive()
    }
}

impl SpaceElement for WindowElements {
    fn geometry(&self) -> Rectangle<i32, Logical> {
        let mut geometry = self.window.geometry();
        if self.window.decorations().is_some() {
            geometry.size.h += DECORATIONS_SIZE as i32;
        }
        geometry
    }

    fn bbox(&self) -> Rectangle<i32, Logical> {
        let mut bbox = self.window.bbox();
        if self.window.decorations().is_some() {
            bbox.size.h += DECORATIONS_SIZE as i32;
        }
        bbox
    }

    fn is_in_input_region(&self, point: &Point<f64, Logical>) -> bool {
        if self.window.decorations().is_some() {
            point.y < DECORATIONS_SIZE as f64
                || self
                    .window
                    .is_in_input_region(&(*point - Point::from((0.0, DECORATIONS_SIZE as f64))))
        } else {
            self.window.is_in_input_region(point)
        }
    }

    fn z_index(&self) -> u8 {
        self.window.z_index()
    }

    fn set_activate(&self, activated: bool) {
        self.window.set_activate(activated);
    }

    fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
        self.window.output_enter(output, overlap);
    }

    fn output_leave(&self, output: &Output) {
        self.window.output_leave(output);
    }

    fn refresh(&self) {
        self.window.refresh();
    }
}

impl PointerTarget<PocoWM> for WindowElements {
    fn enter(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, event: &pointer::MotionEvent) {
        self.target_state.write().unwrap().pointer_location = Some(event.location);
        if self.decorations().is_some() && event.location.y < DECORATIONS_SIZE as f64 {
            return;
        } else if let Some(wl_surface) = self.wl_surface() {
            let mut event = event.clone();
            if self.decorations().is_some() {
                event.location.y -= DECORATIONS_SIZE as f64;
            }
            PointerTarget::<PocoWM>::enter(wl_surface.as_ref(), seat, data, &event);
        }
    }

    fn motion(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, event: &pointer::MotionEvent) {
        self.target_state.write().unwrap().pointer_location = Some(event.location);
        if self.decorations().is_some() && event.location.y < DECORATIONS_SIZE as f64 {
            return;
        } else if let Some(wl_surface) = self.wl_surface() {
            let mut event = event.clone();
            if self.decorations().is_some() {
                event.location.y -= DECORATIONS_SIZE as f64;
            }
            PointerTarget::<PocoWM>::motion(wl_surface.as_ref(), seat, data, &event);
        }
    }

    fn relative_motion(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::RelativeMotionEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::relative_motion(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn button(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, event: &pointer::ButtonEvent) {
        let Some(loc) = self.target_state.read().unwrap().pointer_location else {
            return;
        };

        let mut dir = Edge::empty();
        if loc.x < RESIZE_GRAB_SIZE as f64 {
            dir |= Edge::LEFT;
        }
        if loc.y < RESIZE_GRAB_SIZE as f64 {
            dir |= Edge::TOP;
        }
        if loc.x > self.window.geometry().size.w as f64 - RESIZE_GRAB_SIZE as f64 {
            dir |= Edge::RIGHT;
        }
        if loc.y > self.window.geometry().size.h as f64 - RESIZE_GRAB_SIZE as f64 {
            dir |= Edge::BOTTOM;
        }
        if !dir.is_empty() {
            if let Some(xdg) = self.toplevel() {
                let seat = seat.clone();
                let xdg = xdg.clone();
                let serial = event.serial;
                data.loop_handle.insert_idle(move |data| {
                    data.xdg_resize_request(&xdg, &seat, serial, dir);
                });
                return;
            }
        }

        if self.decorations().is_some() && loc.y < DECORATIONS_SIZE as f64 {
            match self.decorations.borrow().get_button(loc) {
                Some(decorations::Button::Close) => {
                    self.toplevel().map(|t| t.send_close());
                }
                Some(decorations::Button::Maximize) => {
                    // window.set_state(WindowState::Maximized);
                    // if let Some(xdg) = window.toplevel() {
                    //     data.maximize_request(xdg.clone());
                    // }
                }
                Some(decorations::Button::Minimize) => {
                    // window.set_state(WindowState::Minimized);
                }
                None => {
                    if let Some(xdg) = self.toplevel() {
                        // data.xdg_move_request(xdg, &seat.clone(), event.serial);
                        let seat = seat.clone();
                        let xdg = xdg.clone();
                        let serial = event.serial;
                        data.loop_handle.insert_idle(move |data| {
                            data.xdg_move_request(&xdg, &seat, serial);
                        });
                    }
                }
            }
        } else if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::button(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn axis(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, frame: pointer::AxisFrame) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::axis(wl_surface.as_ref(), seat, data, frame);
        }
    }

    fn frame(&self, seat: &Seat<PocoWM>, data: &mut PocoWM) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::frame(wl_surface.as_ref(), seat, data);
        }
    }

    fn leave(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, serial: Serial, time: u32) {
        self.target_state.write().unwrap().pointer_location = None;
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::leave(wl_surface.as_ref(), seat, data, serial, time);
        }
    }

    fn gesture_swipe_begin(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GestureSwipeBeginEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_swipe_begin(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn gesture_swipe_update(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GestureSwipeUpdateEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_swipe_update(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn gesture_swipe_end(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GestureSwipeEndEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_swipe_end(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn gesture_pinch_begin(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GesturePinchBeginEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_pinch_begin(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn gesture_pinch_update(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GesturePinchUpdateEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_pinch_update(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn gesture_pinch_end(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GesturePinchEndEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_pinch_end(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn gesture_hold_begin(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GestureHoldBeginEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_hold_begin(wl_surface.as_ref(), seat, data, event);
        }
    }

    fn gesture_hold_end(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &pointer::GestureHoldEndEvent,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            PointerTarget::<PocoWM>::gesture_hold_end(wl_surface.as_ref(), seat, data, event);
        }
    }
}

impl TouchTarget<PocoWM> for WindowElements {
    fn down(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, event: &touch::DownEvent, seq: Serial) {
        if let Some(wl_surface) = self.wl_surface() {
            TouchTarget::<PocoWM>::down(wl_surface.as_ref(), seat, data, event, seq);
        }
    }

    fn up(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, event: &touch::UpEvent, seq: Serial) {
        if let Some(wl_surface) = self.wl_surface() {
            TouchTarget::<PocoWM>::up(wl_surface.as_ref(), seat, data, event, seq);
        }
    }

    fn motion(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &touch::MotionEvent,
        seq: Serial,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            TouchTarget::<PocoWM>::motion(wl_surface.as_ref(), seat, data, event, seq);
        }
    }

    fn frame(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, seq: Serial) {
        if let Some(wl_surface) = self.wl_surface() {
            TouchTarget::<PocoWM>::frame(wl_surface.as_ref(), seat, data, seq);
        }
    }

    fn cancel(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, seq: Serial) {
        if let Some(wl_surface) = self.wl_surface() {
            TouchTarget::<PocoWM>::cancel(wl_surface.as_ref(), seat, data, seq);
        }
    }

    fn shape(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &touch::ShapeEvent,
        seq: Serial,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            TouchTarget::<PocoWM>::shape(wl_surface.as_ref(), seat, data, event, seq);
        }
    }

    fn orientation(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        event: &touch::OrientationEvent,
        seq: Serial,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            TouchTarget::<PocoWM>::orientation(wl_surface.as_ref(), seat, data, event, seq);
        }
    }
}

impl KeyboardTarget<PocoWM> for WindowElements {
    fn enter(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        keys: Vec<KeysymHandle<'_>>,
        serial: Serial,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            KeyboardTarget::<PocoWM>::enter(wl_surface.as_ref(), seat, data, keys, serial);
        }
    }

    fn leave(&self, seat: &Seat<PocoWM>, data: &mut PocoWM, serial: Serial) {
        if let Some(wl_surface) = self.wl_surface() {
            KeyboardTarget::<PocoWM>::leave(wl_surface.as_ref(), seat, data, serial);
        }
    }

    fn key(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        key: KeysymHandle<'_>,
        state: KeyState,
        serial: Serial,
        time: u32,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            KeyboardTarget::<PocoWM>::key(
                wl_surface.as_ref(),
                seat,
                data,
                key,
                state,
                serial,
                time,
            );
        }
    }

    fn modifiers(
        &self,
        seat: &Seat<PocoWM>,
        data: &mut PocoWM,
        modifiers: keyboard::ModifiersState,
        serial: Serial,
    ) {
        if let Some(wl_surface) = self.wl_surface() {
            KeyboardTarget::<PocoWM>::modifiers(wl_surface.as_ref(), seat, data, modifiers, serial);
        }
    }
}

impl WaylandFocus for WindowElements {
    #[inline]
    fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
        self.window.wl_surface()
    }
}

impl<R> AsRenderElements<R> for WindowElements
where
    R: Renderer + ImportAll + ImportMem,
    <R as Renderer>::TextureId: Clone + Texture + 'static,
{
    type RenderElement = WindowRenderElement<R>;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut R,
        mut location: Point<i32, Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        self.decorations.borrow_mut().redraw(&self.window);
        let decorations = if self.window.decorations().is_some() {
            let d = self
                .decorations
                .borrow()
                .render_elements(renderer, location, scale, alpha)
                .into_iter()
                .map(C::from)
                .collect::<Vec<_>>();
            location.y += (scale.y * DECORATIONS_SIZE as f64) as i32;
            d
        } else {
            vec![]
        };
        let window = self
            .window
            .render_elements(renderer, location, scale, alpha)
            .into_iter()
            .map(C::from)
            .collect::<Vec<_>>();
        // let mut elements =
        //     AsRenderElements::<R>::render_elements(&self.window, renderer, location, scale, alpha);
        let mut elements = window;
        elements.extend(decorations);
        elements
    }
}
