use std::cell::RefCell;

use crate::window::Window;
use smithay::backend::renderer::element::solid::{SolidColorBuffer, SolidColorRenderElement};
use smithay::backend::renderer::element::{AsRenderElements, Kind};
use smithay::backend::renderer::Renderer;
use smithay::render_elements;
use smithay::utils::{Logical, Physical, Point, Scale};

pub const DECORATIONS_HEIGHT: u32 = BUTTON_SIZE + 2 * BUTTON_GAP;
pub const BACKGROUND_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
pub const CLOSE_BUTTON_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
pub const MAXIMIZE_BUTTON_COLOR: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
pub const MINIMIZE_BUTTON_COLOR: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
pub const BUTTON_SIZE: u32 = 16;
pub const BUTTON_GAP: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Button {
    Close,
    Maximize,
    Minimize,
}

#[derive(Debug, Clone, Default)]
struct DecorationsBuffers {
    background: SolidColorBuffer,
    close_button: SolidColorBuffer,
    maximize_button: SolidColorBuffer,
    minimize_button: SolidColorBuffer,
}

impl DecorationsBuffers {
    pub fn update(&mut self, decorations: &Decorations) {
        self.background.update(
            (
                decorations.window.geometry().size.w,
                DECORATIONS_HEIGHT as i32,
            ),
            BACKGROUND_COLOR,
        );
        self.close_button
            .update((BUTTON_SIZE as i32, BUTTON_SIZE as i32), CLOSE_BUTTON_COLOR);
        self.maximize_button.update(
            (BUTTON_SIZE as i32, BUTTON_SIZE as i32),
            MAXIMIZE_BUTTON_COLOR,
        );
        self.minimize_button.update(
            (BUTTON_SIZE as i32, BUTTON_SIZE as i32),
            MINIMIZE_BUTTON_COLOR,
        );
    }
}

#[derive(Debug, Clone)]
pub struct Decorations {
    window: Window,
    buffers: RefCell<DecorationsBuffers>,
}

impl PartialEq for Decorations {
    fn eq(&self, other: &Self) -> bool {
        self.window == other.window
    }
}
impl Eq for Decorations {}

impl Decorations {
    pub fn new(window: Window) -> Self {
        Self {
            window,
            buffers: RefCell::new(DecorationsBuffers::default()),
        }
    }
    pub fn get_button(&self, loc: Point<f64, Logical>) -> Option<Button> {
        if loc.x < BUTTON_GAP as f64 || loc.y < BUTTON_GAP as f64 {
            return None;
        }
        if loc.x < BUTTON_SIZE as f64 + BUTTON_GAP as f64
            && loc.y < BUTTON_SIZE as f64 + BUTTON_GAP as f64
        {
            return Some(Button::Close);
        }
        if loc.x < 2.0 * BUTTON_SIZE as f64 + 2.0 * BUTTON_GAP as f64
            && loc.y < BUTTON_SIZE as f64 + BUTTON_GAP as f64
        {
            return Some(Button::Maximize);
        }
        if loc.x < 3.0 * BUTTON_SIZE as f64 + 3.0 * BUTTON_GAP as f64
            && loc.y < BUTTON_SIZE as f64 + BUTTON_GAP as f64
        {
            return Some(Button::Minimize);
        }

        None
    }
}

render_elements! {
    pub DecorationsElement;
    Decorations=SolidColorRenderElement,
}

impl<R> AsRenderElements<R> for Decorations
where
    R: Renderer,
    <R as Renderer>::TextureId: 'static,
{
    type RenderElement = DecorationsElement;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        _renderer: &mut R,
        location: Point<i32, Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        let mut buffers = self.buffers.borrow_mut();
        buffers.update(self);
        vec![
            SolidColorRenderElement::from_buffer(
                &buffers.close_button,
                location + Point::from((BUTTON_GAP as i32, BUTTON_GAP as i32)),
                scale,
                alpha,
                Kind::Unspecified,
            ),
            SolidColorRenderElement::from_buffer(
                &buffers.maximize_button,
                location
                    + Point::from((
                        BUTTON_SIZE as i32 + 2 * BUTTON_GAP as i32,
                        BUTTON_GAP as i32,
                    )),
                scale,
                alpha,
                Kind::Unspecified,
            ),
            SolidColorRenderElement::from_buffer(
                &buffers.minimize_button,
                location
                    + Point::from((
                        2 * BUTTON_SIZE as i32 + 3 * BUTTON_GAP as i32,
                        BUTTON_GAP as i32,
                    )),
                scale,
                alpha,
                Kind::Unspecified,
            ),
            SolidColorRenderElement::from_buffer(
                &buffers.background,
                location,
                scale,
                alpha,
                Kind::Unspecified,
            ),
        ]
        .into_iter()
        .map(DecorationsElement::from)
        .map(C::from)
        .collect::<Vec<_>>()
    }
}
