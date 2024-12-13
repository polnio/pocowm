use smithay::desktop::Window;
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
    pub fn iter_windows<'a>(&'a self) -> Box<dyn Iterator<Item = &Window> + 'a> {
        let iter = self.elements.iter().flat_map(|element| match element {
            LayoutElement::Window(window) => Box::new(std::iter::once(window)),
            LayoutElement::SubLayout(layout) => layout.iter_windows(),
        });
        Box::new(iter)
    }
    pub fn get_window(&self, wl_surface: &WlSurface) -> Option<&Window> {
        self.iter_windows().find(|window| {
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
    pub fn add_element(&mut self, element: LayoutElement, positions: Option<&[usize]>) {
        let Some(positions) = positions else {
            self.elements.push(element);
            return;
        };
        let Some(pos) = positions.first() else {
            self.elements.push(element);
            return;
        };
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
    pub fn remove_window(&mut self, window: &Window) {
        self.remove_window_rec(window);
    }
    fn remove_window_rec(&mut self, window: &Window) -> bool {
        let index = self.elements.iter().position(|element| match element {
            LayoutElement::Window(w) => w == window,
            LayoutElement::SubLayout(_) => false,
        });
        if let Some(index) = index {
            self.elements.remove(index);
            true
        } else {
            for (i, element) in self.elements.iter_mut().enumerate() {
                if let LayoutElement::SubLayout(sl) = element {
                    if sl.remove_window_rec(window) {
                        if sl.elements.is_empty() {
                            self.elements.remove(i);
                        }
                        return true;
                    }
                }
            }
            false
        }
    }
}
