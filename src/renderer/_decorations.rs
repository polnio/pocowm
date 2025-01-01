use crate::window::Window;
use smithay::backend::renderer::element::solid::{SolidColorBuffer, SolidColorRenderElement};
use smithay::backend::renderer::element::{AsRenderElements, Id, Kind};
use smithay::backend::renderer::Renderer;
use smithay::utils::{Logical, Physical, Point, Scale};

pub const DECORATIONS_SIZE: u32 = BUTTON_SIZE + 2 * BUTTON_GAP;
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

#[derive(Debug, Clone)]
pub struct DecorationsElements {
    id: Id,
    background: SolidColorBuffer,
    close_button: SolidColorBuffer,
    maximize_button: SolidColorBuffer,
    minimize_button: SolidColorBuffer,
}

impl PartialEq for DecorationsElements {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for DecorationsElements {}

impl DecorationsElements {
    pub fn new() -> Self {
        Self {
            id: Id::new(),
            background: SolidColorBuffer::default(),
            close_button: SolidColorBuffer::default(),
            maximize_button: SolidColorBuffer::default(),
            minimize_button: SolidColorBuffer::default(),
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
    pub fn redraw(&mut self, window: &Window) {
        self.background.update(
            (window.geometry().size.w, DECORATIONS_SIZE as i32),
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

impl<R: Renderer> AsRenderElements<R> for DecorationsElements {
    type RenderElement = SolidColorRenderElement;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        _renderer: &mut R,
        location: Point<i32, Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        vec![
            SolidColorRenderElement::from_buffer(
                &self.close_button,
                location + Point::from((BUTTON_GAP as i32, BUTTON_GAP as i32)),
                scale,
                alpha,
                Kind::Unspecified,
            )
            .into(),
            SolidColorRenderElement::from_buffer(
                &self.maximize_button,
                location
                    + Point::from((
                        BUTTON_SIZE as i32 + 2 * BUTTON_GAP as i32,
                        BUTTON_GAP as i32,
                    )),
                scale,
                alpha,
                Kind::Unspecified,
            )
            .into(),
            SolidColorRenderElement::from_buffer(
                &self.minimize_button,
                location
                    + Point::from((
                        2 * BUTTON_SIZE as i32 + 3 * BUTTON_GAP as i32,
                        BUTTON_GAP as i32,
                    )),
                scale,
                alpha,
                Kind::Unspecified,
            )
            .into(),
            SolidColorRenderElement::from_buffer(
                &self.background,
                location,
                scale,
                alpha,
                Kind::Unspecified,
            )
            .into(),
        ]
    }
}
