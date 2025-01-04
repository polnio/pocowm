use crate::utils::Edge;
use crate::window::{Window, WindowState};
use crate::PocoWM;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::seat::WaylandFocus as _;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LayoutType {
    #[default]
    Horizontal,
    Vertical,
    Tabbed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutElement {
    Window(Window),
    SubLayout(Layout),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Layout {
    pub last_focused: usize,
    pub layout_type: LayoutType,
    pub elements: Vec<LayoutElement>,
}

impl Layout {
    pub fn new(layout_type: LayoutType) -> Self {
        Self {
            layout_type,
            ..Default::default()
        }
    }
    pub fn iter_windows<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Window> + 'a> {
        let iter = self.elements.iter().flat_map(|element| match element {
            LayoutElement::Window(window) => Box::new(std::iter::once(window)),
            LayoutElement::SubLayout(layout) => layout.iter_windows(),
        });
        Box::new(iter)
    }
    pub fn iter_mut_windows<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut Window> + 'a> {
        let iter = self.elements.iter_mut().flat_map(|element| match element {
            LayoutElement::Window(window) => Box::new(std::iter::once(window)),
            LayoutElement::SubLayout(layout) => layout.iter_mut_windows(),
        });
        Box::new(iter)
    }
    pub fn get_window_from_surface(&self, wl_surface: &WlSurface) -> Option<&Window> {
        self.iter_windows().find(|window| {
            window
                .wl_surface()
                .is_some_and(|s| s.as_ref() == wl_surface)
        })
    }
    pub fn get_mut_window_from_surface(&mut self, wl_surface: &WlSurface) -> Option<&mut Window> {
        self.iter_mut_windows().find(|window| {
            window
                .wl_surface()
                .is_some_and(|s| s.as_ref() == wl_surface)
        })
    }
    pub fn get_window_positions(&self, window: &Window) -> Option<Vec<usize>> {
        self.elements.iter().enumerate().find_map(|(i, e)| match e {
            LayoutElement::Window(w) => (w == window).then(|| vec![i]),
            LayoutElement::SubLayout(sl) => sl.get_window_positions(window).map(|mut v| {
                v.insert(0, i);
                v
            }),
        })
    }
    pub fn get_window(&self, positions: &[usize]) -> Option<&Window> {
        if self.elements.is_empty() {
            return None;
        }
        let Some(mut pos) = positions.first().copied() else {
            return match self.elements.first() {
                Some(LayoutElement::Window(w)) => Some(w),
                _ => None,
            };
        };
        if pos >= self.elements.len() {
            pos = self.elements.len() - 1;
        }
        match self.elements.get(pos)? {
            LayoutElement::Window(w) if positions.len() == 1 => Some(w),
            LayoutElement::Window(_) => self.get_window(&[pos.saturating_sub(1)]),
            LayoutElement::SubLayout(sl) => sl.get_window(&positions[1..]),
        }
    }
    pub fn add_element(&mut self, element: LayoutElement, positions: Option<&[usize]>) {
        let Some(positions) = positions else {
            self.elements.push(element);
            return;
        };
        let Some(pos) = positions.first() else {
            self.elements.push(element);
            return;
        };
        if positions.len() == 1 {
            self.elements.insert(*pos, element);
            return;
        }
        match self.elements.get_mut(*pos) {
            Some(LayoutElement::SubLayout(sl)) => {
                sl.add_element(element, Some(&positions[1..]));
            }
            _ => {
                self.elements
                    .insert((*pos).min(self.elements.len()), element);
            }
        }
    }
    pub fn add_window(&mut self, window: Window, positions: Option<&[usize]>) {
        self.add_element(LayoutElement::Window(window), positions);
    }
    pub fn add_sublayout(&mut self, layout: Layout, positions: Option<&[usize]>) {
        self.add_element(LayoutElement::SubLayout(layout), positions);
    }
    pub fn remove_window(&mut self, positions: Option<&[usize]>) -> Option<Window> {
        self.remove_window_rec(positions)
    }
    fn remove_window_rec(&mut self, positions: Option<&[usize]>) -> Option<Window> {
        let positions = positions?;
        let pos = positions.first()?;
        let el = self.elements.get_mut(*pos)?;
        match el {
            LayoutElement::Window(_) => {
                let LayoutElement::Window(w) = self.elements.remove(*pos) else {
                    unreachable!();
                };
                Some(w)
            }
            LayoutElement::SubLayout(sl) => {
                let removed = sl.remove_window_rec(Some(&positions[1..]));
                if removed.is_some() {
                    if sl.elements.is_empty() {
                        self.elements.remove(*pos);
                    }
                }
                removed
            }
        }
    }

    pub fn get_last_focused(&self) -> Option<&Window> {
        self.elements.get(self.last_focused).and_then(|e| match e {
            LayoutElement::Window(w) => Some(w),
            LayoutElement::SubLayout(sl) => sl.get_last_focused(),
        })
    }

    pub fn get_window_neighbor(&self, window: &Window, edge: Edge) -> Option<&Window> {
        let pos = self.get_window_positions(window)?;
        self.get_window_neighbor_rec(&pos, edge)
    }
    pub fn get_window_neighbor_rec(&self, pos: &[usize], edge: Edge) -> Option<&Window> {
        let p = pos.first()?;
        let el = self.elements.get(*p)?;
        match el {
            LayoutElement::SubLayout(sl) => {
                if let Some(p) = pos.get(1..) {
                    if let Some(w) = sl.get_window_neighbor_rec(p, edge) {
                        return Some(w);
                    }
                };
            }
            _ => {}
        };

        let node = match (&self.layout_type, edge) {
            (LayoutType::Horizontal, Edge::LEFT) => self.elements.get(p.saturating_sub(1))?,
            (LayoutType::Horizontal, Edge::RIGHT) => self.elements.get(p.saturating_add(1))?,
            (LayoutType::Vertical, Edge::TOP) => self.elements.get(p.saturating_sub(1))?,
            (LayoutType::Vertical, Edge::BOTTOM) => self.elements.get(p.saturating_add(1))?,
            _ => return None,
        };
        match node {
            LayoutElement::Window(w) => Some(w),
            LayoutElement::SubLayout(sl) => sl.get_last_focused(),
        }
    }

    pub fn add_window_neighbor(
        &mut self,
        new_window: Window,
        pos: &[usize],
        edge: Edge,
    ) -> Option<()> {
        let p = pos.first()?;
        let el = self.elements.get_mut(*p)?;
        match el {
            LayoutElement::SubLayout(sl) => {
                if let Some(p) = pos.get(1..) {
                    if sl
                        .add_window_neighbor(new_window.clone(), p, edge)
                        .is_some()
                    {
                        return Some(());
                    }
                };
            }
            _ => {}
        };

        let edge = match &self.layout_type {
            LayoutType::Horizontal => edge.intersection(Edge::LEFT | Edge::RIGHT),
            LayoutType::Vertical => edge.intersection(Edge::TOP | Edge::BOTTOM),
            LayoutType::Tabbed => edge,
        };

        let index = match (&self.layout_type, edge) {
            (LayoutType::Horizontal, Edge::LEFT) => *p,
            (LayoutType::Horizontal, Edge::RIGHT) => p.saturating_add(1),
            (LayoutType::Vertical, Edge::TOP) => *p,
            (LayoutType::Vertical, Edge::BOTTOM) => p.saturating_add(1),
            _ => return None,
        };
        self.elements
            .insert(index, LayoutElement::Window(new_window));
        Some(())
    }

    pub fn on_focus(&mut self, window: &Window) {
        if let Some(i) = self.on_focus_rec(window) {
            self.last_focused = i;
        }
    }

    fn on_focus_rec(&mut self, window: &Window) -> Option<usize> {
        let index = self
            .elements
            .iter_mut()
            .enumerate()
            .find_map(|(i, e)| match e {
                LayoutElement::Window(w) if w == window => Some(i),
                LayoutElement::Window(_) => None,
                LayoutElement::SubLayout(sl) => sl.on_focus_rec(window),
            });
        if let Some(i) = index {
            self.last_focused = i;
        }
        index
    }
}

impl PocoWM {
    pub fn switch_to_layout(&mut self, layout_type: LayoutType) {
        let focused_window = self.seat.get_keyboard().and_then(|k| k.current_focus());
        if let Some(focused_window) = focused_window {
            if focused_window.state().contains(WindowState::FLOATING) {
                return;
            }
            let mut layout = Layout::new(layout_type);
            let positions = self.layout.get_window_positions(&focused_window);
            self.layout.remove_window(positions.as_deref());
            layout.add_window(focused_window, None);
            self.layout.add_sublayout(layout, positions.as_deref());
        } else if self.layout.elements.is_empty() {
            self.layout.layout_type = layout_type;
        } else {
            let mut layout = Layout::new(layout_type);
            std::mem::swap(&mut self.layout, &mut layout);
            self.layout.add_sublayout(layout, None);
        }
    }
    pub fn toggle_floating(&mut self) {
        let focused_window = self.seat.get_keyboard().and_then(|k| k.current_focus());
        let Some(focused_window) = focused_window else {
            return;
        };
        focused_window.state_mut().toggle(WindowState::FLOATING);
        self.renderer.render(&self.layout);
    }
}
