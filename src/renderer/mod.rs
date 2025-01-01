// https://danyspin97.org/talks/writing-a-wayland-wallpaper-daemon-in-rust/#47
use crate::layout::{Layout, LayoutElement, LayoutType};
use crate::window::{Window, WindowState};
use smithay::desktop::Space;
use smithay::utils::{Logical, Rectangle};
use std::ops::{Deref, DerefMut};

const GAP: i32 = 20;

#[derive(Debug, Default, PartialEq)]
pub struct Renderer {
    pub space: Space<Window>,
}

impl Renderer {
    pub fn render(&mut self, layout: &Layout) -> Option<()> {
        let output = self.space.outputs().next()?;
        let full_rect = self.space.output_geometry(output)?;
        let mut rect = full_rect;
        rect.loc.x += GAP;
        rect.loc.y += GAP;
        rect.size.w -= GAP * 2;
        rect.size.h -= GAP * 2;
        self.render_rec(&layout, rect)?;
        layout.iter_windows().for_each(|window| {
            if window.state().contains(WindowState::MINIMIZED) {
                self.unmap_elem(window);
            } else if window.state().contains(WindowState::MAXIMIZED) {
                self.render_window(window, full_rect);
            } else if window.state().contains(WindowState::FLOATING) {
                self.render_window(window, *window.floating_rect());
            }
        });
        Some(())
    }

    fn render_rec(&mut self, layout: &Layout, rect: Rectangle<i32, Logical>) -> Option<()> {
        let elements = layout.elements.iter().filter(|e| match e {
            LayoutElement::Window(w) => w.state().is_empty(),
            _ => true,
        });
        let elements_count = elements.clone().count() as i32;
        elements.enumerate().try_for_each(|(i, element)| {
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
                LayoutElement::Window(window) => self.render_window(window, rect),
                LayoutElement::SubLayout(layout) => self.render_rec(layout, rect),
            }
        });

        Some(())
    }

    pub fn render_window(&mut self, window: &Window, rect: Rectangle<i32, Logical>) -> Option<()> {
        window.resize(rect.size);
        self.space.map_element(window.clone(), rect.loc, false);
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
