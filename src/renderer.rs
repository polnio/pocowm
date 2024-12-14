use crate::layout::{Layout, LayoutElement, LayoutType};
// use smithay::backend::renderer::element::{AsRenderElements, Element, Id, RenderElement};
// use smithay::backend::renderer::utils::CommitCounter;
// use smithay::backend::renderer::{Color32F, Frame, Renderer as SmRenderer, Texture};
use smithay::desktop::{Space, Window, WindowSurfaceType};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
// use smithay::utils::{Buffer, Logical, Physical, Point, Rectangle, Scale};
use smithay::utils::{Logical, Point, Rectangle};
use std::ops::{Deref, DerefMut};

/* struct TabHeader<'a> {
    id: Id,
    geometry: Rectangle<i32, Physical>,
    window: &'a Window,
}
struct TabHeaderElement {
    id: Id,
    src: Rectangle<f64, Buffer>,
    geometry: Rectangle<i32, Physical>,
    commit: CommitCounter,
}
impl Element for TabHeaderElement {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.commit
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        self.src
    }

    fn geometry(&self, _scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.geometry
    }
}
impl<R> RenderElement<R> for TabHeaderElement
where
    R: SmRenderer,
{
    fn draw(
        &self,
        frame: &mut R::Frame<'_>,
        _src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        _opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), R::Error> {
        frame.draw_solid(dst, damage, Color32F::new(255.0, 255.0, 255.0, 0.0))
    }
}

impl<T, R> AsRenderElements<R> for TabHeader<'_>
where
    T: Texture,
    R: SmRenderer<TextureId = T>,
{
    type RenderElement = TabHeaderElement;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut R,
        location: Point<i32, Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        vec![TabHeaderElement { id: self.id }]
    }
} */

const GAP: i32 = 20;

#[derive(Debug, Default, PartialEq)]
pub struct Renderer {
    pub space: Space<Window>,
}

impl Renderer {
    // pub fn new() -> Self {
    //     Self::default()
    // }

    pub fn surface_under(
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

    pub fn render(&mut self, layout: &Layout) -> Option<()> {
        let output = self.space.outputs().next()?;
        let mut rect = self.space.output_geometry(output)?;
        rect.loc.x += GAP;
        rect.loc.y += GAP;
        rect.size.w -= GAP * 2;
        rect.size.h -= GAP * 2;
        self.render_rec(layout, rect)
    }

    fn render_rec(&mut self, layout: &Layout, rect: Rectangle<i32, Logical>) -> Option<()> {
        let elements_count = layout.elements.len() as i32;
        layout
            .elements
            .iter()
            .enumerate()
            .try_for_each(|(i, element)| {
                let i = i as i32;
                let mut rect = rect.clone();
                match layout.layout_type {
                    LayoutType::Horizontal => {
                        rect.size.w = (rect.size.w - GAP * (elements_count - 1)) / elements_count;
                        // rect.size.w = (rect.size.w + GAP) / elements_count - GAP;
                        rect.loc.x += (rect.size.w + GAP) * i;
                    }
                    LayoutType::Vertical => {
                        rect.size.h = (rect.size.h - GAP * (elements_count - 1)) / elements_count;
                        // rect.size.h = (rect.size.h + GAP) / elements_count - GAP;
                        rect.loc.y += (rect.size.h + GAP) * i;
                    }
                    LayoutType::Tabbed => {}
                }

                match element {
                    LayoutElement::Window(window) => {
                        let xdg = window.toplevel()?;
                        xdg.with_pending_state(|state| {
                            state.size = Some(rect.size);
                        });
                        xdg.send_configure();
                        self.space.map_element(window.clone(), rect.loc, false);
                    }
                    LayoutElement::SubLayout(layout) => {
                        self.render_rec(layout, rect)?;
                    }
                }

                Some(())
            });

        Some(())
    }
}

impl Deref for Renderer {
    type Target = Space<Window>;

    fn deref(&self) -> &Self::Target {
        &self.space
    }
}

impl DerefMut for Renderer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.space
    }
}
