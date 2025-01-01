use super::borders::{BordersElement, BORDER_SIZE};
use super::decorations::{DecorationsElement, DECORATIONS_HEIGHT};
use super::Window;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::element::AsRenderElements;
use smithay::backend::renderer::{ImportAll, ImportMem, Renderer, Texture};
use smithay::desktop::space::SpaceElement;
use smithay::output::Output;
use smithay::render_elements;
use smithay::utils::{IsAlive, Logical, Physical, Point, Rectangle, Scale};

impl Window {
    // Bypass InnerWindow::geometry
    pub fn geometry(&self) -> Rectangle<i32, Logical> {
        SpaceElement::geometry(self)
    }
}

impl IsAlive for Window {
    fn alive(&self) -> bool {
        self.inner().alive()
    }
}

impl SpaceElement for Window {
    fn geometry(&self) -> Rectangle<i32, Logical> {
        let mut geometry = self.inner().geometry();
        if self.decorations().is_some() {
            geometry.size.h += DECORATIONS_HEIGHT as i32;
        };
        geometry
    }

    fn bbox(&self) -> Rectangle<i32, Logical> {
        let mut bbox = self.inner().bbox();
        if self.decorations().is_some() {
            bbox.size.h += DECORATIONS_HEIGHT as i32;
        }
        if self.is_focused() {
            bbox.loc.x -= BORDER_SIZE as i32;
            bbox.loc.y -= BORDER_SIZE as i32;
            bbox.size.w += 2 * BORDER_SIZE as i32;
            bbox.size.h += 2 * BORDER_SIZE as i32;
        }
        bbox
    }

    fn is_in_input_region(&self, point: &Point<f64, Logical>) -> bool {
        if self.is_focused() && !self.borders().get_edge(point).is_empty() {
            return true;
        }
        if self.decorations().is_some() && point.y < DECORATIONS_HEIGHT as f64 {
            return true;
        }
        if self.decorations().is_some() {
            self.inner()
                .is_in_input_region(&(*point - Point::from((0.0, DECORATIONS_HEIGHT as f64))))
        } else {
            self.inner().is_in_input_region(point)
        }
    }

    fn z_index(&self) -> u8 {
        self.inner().z_index()
    }

    fn set_activate(&self, activated: bool) {
        self.inner().set_activate(activated);
    }

    fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
        self.inner().output_enter(output, overlap);
    }

    fn output_leave(&self, output: &Output) {
        self.inner().output_leave(output);
    }

    fn refresh(&self) {
        self.inner().refresh();
    }
}

render_elements! {
    pub WindowElement<R> where R: ImportAll + ImportMem;
    Window=WaylandSurfaceRenderElement<R>,
    // DecorationsAndBorders=SolidColorRenderElement,
    Decorations=DecorationsElement,
    Borders=BordersElement,
}

impl<R> AsRenderElements<R> for Window
where
    R: Renderer + ImportAll + ImportMem,
    <R as Renderer>::TextureId: Clone + Texture + 'static,
{
    type RenderElement = WindowElement<R>;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut R,
        mut location: Point<i32, Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        let decorations = self
            .decorations()
            .as_ref()
            .map(|d| {
                d.render_elements(renderer, location, scale, alpha)
                    .into_iter()
                    .map(C::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let borders = self
            .is_focused()
            .then(|| {
                self.borders()
                    .render_elements(renderer, location, scale, alpha)
                    .into_iter()
                    .map(C::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if self.decorations().is_some() {
            location.y += (scale.y * DECORATIONS_HEIGHT as f64) as i32;
        }

        let window = self
            .inner()
            .render_elements(renderer, location, scale, alpha)
            .into_iter()
            .map(C::from)
            .collect::<Vec<_>>();
        // let mut elements =
        //     AsRenderElements::<R>::render_elements(&self.window, renderer, location, scale, alpha);
        let mut elements = window;
        elements.extend(decorations);
        elements.extend(borders);
        elements
    }
}
