use crate::utils::Edge;

use super::Window;
use smithay::backend::renderer::element::solid::{SolidColorBuffer, SolidColorRenderElement};
use smithay::backend::renderer::element::{AsRenderElements, Kind};
use smithay::backend::renderer::Renderer;
use smithay::render_elements;
use smithay::utils::{Logical, Physical, Point, Scale};
use std::cell::RefCell;

pub const BORDER_SIZE: u32 = 10;
pub const BORDER_COLOR: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

#[derive(Debug, Clone, Default)]
struct BordersBuffers {
    top: SolidColorBuffer,
    bottom: SolidColorBuffer,
    left: SolidColorBuffer,
    right: SolidColorBuffer,
}

impl BordersBuffers {
    pub fn update(&mut self, borders: &Borders) {
        let width = borders.window.geometry().size.w + 2 * BORDER_SIZE as i32;
        let height = borders.window.geometry().size.h + 2 * BORDER_SIZE as i32;
        self.top.update((width, BORDER_SIZE as i32), BORDER_COLOR);
        self.bottom
            .update((width, BORDER_SIZE as i32), BORDER_COLOR);
        self.left.update((BORDER_SIZE as i32, height), BORDER_COLOR);
        self.right
            .update((BORDER_SIZE as i32, height), BORDER_COLOR);
    }
}

#[derive(Debug, Clone)]
pub struct Borders {
    window: Window,
    buffers: RefCell<BordersBuffers>,
}

impl PartialEq for Borders {
    fn eq(&self, other: &Self) -> bool {
        self.window == other.window
    }
}
impl Eq for Borders {}

impl Borders {
    pub fn new(window: Window) -> Self {
        Self {
            window,
            buffers: RefCell::new(BordersBuffers::default()),
        }
    }
    pub fn get_edge(&self, loc: &Point<f64, Logical>) -> Edge {
        let mut edges = Edge::empty();
        if loc.x < 0 as f64 {
            edges |= Edge::LEFT;
        }
        if loc.y < 0 as f64 {
            edges |= Edge::TOP;
        }
        if loc.x > self.window.geometry().size.w as f64 - 0 as f64 {
            edges |= Edge::RIGHT;
        }
        if loc.y > self.window.geometry().size.h as f64 - 0 as f64 {
            edges |= Edge::BOTTOM;
        }
        edges
    }
}

render_elements! {
    pub BordersElement;
    Borders=SolidColorRenderElement,
}

impl<R> AsRenderElements<R> for Borders
where
    R: Renderer,
    <R as Renderer>::TextureId: 'static,
{
    type RenderElement = BordersElement;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        _renderer: &mut R,
        mut location: Point<i32, Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        let mut buffers = self.buffers.borrow_mut();
        buffers.update(self);
        location += self
            .window
            .geometry()
            .loc
            .to_f64()
            .to_physical(scale)
            .to_i32_round();
        vec![
            SolidColorRenderElement::from_buffer(
                &buffers.top,
                location - Point::from((BORDER_SIZE as i32, BORDER_SIZE as i32)),
                scale,
                alpha,
                Kind::Unspecified,
            ),
            SolidColorRenderElement::from_buffer(
                &buffers.bottom,
                location
                    + Point::from((0 - BORDER_SIZE as i32, self.window.geometry().size.h as i32)),
                scale,
                alpha,
                Kind::Unspecified,
            ),
            SolidColorRenderElement::from_buffer(
                &buffers.left,
                location - Point::from((BORDER_SIZE as i32, BORDER_SIZE as i32)),
                scale,
                alpha,
                Kind::Unspecified,
            ),
            SolidColorRenderElement::from_buffer(
                &buffers.right,
                location
                    + Point::from((self.window.geometry().size.w as i32, 0 - BORDER_SIZE as i32)),
                scale,
                alpha,
                Kind::Unspecified,
            ),
        ]
        .into_iter()
        .map(BordersElement::from)
        .map(C::from)
        .collect::<Vec<_>>()
    }
}
