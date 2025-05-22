use crate::utils::Edge;
use crate::window::{Window as InnerWindow, WindowState};
use crate::PocoWM;
use derive_more::Deref;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::seat::WaylandFocus;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Id(usize);
impl Id {
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
    pub fn prev(self) -> Self {
        Self(self.0 - 1)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LayoutType {
    #[default]
    Horizontal,
    Vertical,
    Tabbed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubLayout {
    pub id: Id,
    pub parent: Option<Id>,
    pub children: Vec<Id>,
    pub last_focused: usize,
    pub layout_type: LayoutType,
}

#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub struct Window {
    pub id: Id,
    pub parent: Id,
    #[deref]
    pub inner: InnerWindow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutElement {
    SubLayout(SubLayout),
    Window(Window),
}
#[allow(dead_code)]
impl LayoutElement {
    #[inline]
    fn id(&self) -> Id {
        match self {
            Self::SubLayout(sl) => sl.id,
            Self::Window(w) => w.id,
        }
    }
    #[inline]
    fn set_id(&mut self, id: Id) {
        match self {
            Self::SubLayout(sl) => sl.id = id,
            Self::Window(w) => w.id = id,
        }
    }
    #[inline]
    fn parent(&self) -> Option<Id> {
        match self {
            Self::SubLayout(sl) => sl.parent,
            Self::Window(w) => Some(w.parent),
        }
    }
    #[inline]
    fn set_parent(&mut self, parent: Id) {
        match self {
            Self::SubLayout(sl) => sl.parent = Some(parent),
            Self::Window(w) => w.parent = parent,
        }
    }
    #[inline]
    pub fn get_sublayout(&self) -> Option<&SubLayout> {
        match self {
            Self::SubLayout(sl) => Some(sl),
            _ => None,
        }
    }
    #[inline]
    pub fn get_sublayout_mut(&mut self) -> Option<&mut SubLayout> {
        match self {
            Self::SubLayout(sl) => Some(sl),
            _ => None,
        }
    }
    #[inline]
    pub fn get_window(&self) -> Option<&Window> {
        match self {
            Self::Window(w) => Some(w),
            _ => None,
        }
    }
    #[inline]
    pub fn get_window_mut(&mut self) -> Option<&mut Window> {
        match self {
            Self::Window(w) => Some(w),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    // root: SubLayout,
    next_id: Id,
    elements: HashMap<Id, LayoutElement>,
}

impl Layout {
    pub fn new() -> Self {
        let root_id = Id::default();
        let mut elements = HashMap::default();
        elements.insert(root_id, LayoutElement::SubLayout(SubLayout::default()));
        let next_id = root_id.next();
        Self { next_id, elements }
    }
    pub fn root(&self) -> &SubLayout {
        self.elements
            .get(&Id::default())
            .unwrap()
            .get_sublayout()
            .unwrap()
    }
    pub fn root_mut(&mut self) -> &mut SubLayout {
        self.elements
            .get_mut(&Id::default())
            .unwrap()
            .get_sublayout_mut()
            .unwrap()
    }
    #[inline]
    fn iter_pairs(&self) -> impl Iterator<Item = (Id, &LayoutElement)> {
        self.elements.iter().map(|(i, e)| (*i, e))
    }
    #[inline]
    pub fn iter_windows(&self) -> impl Iterator<Item = &InnerWindow> {
        self.elements.iter().filter_map(|(_, e)| match e {
            LayoutElement::Window(w) => Some(&w.inner),
            LayoutElement::SubLayout(_) => None,
        })
    }
    #[inline]
    pub fn remove_element(&mut self, id: Id) -> Option<LayoutElement> {
        let (parent, is_empty) = self
            .get_parent(id)
            .and_then(|p| self.get_sublayout_mut(p))
            .map(|parent| {
                parent.children.retain(|i| i != &id);
                (parent.id, parent.children.is_empty())
            })?;
        let el = self.elements.remove(&id)?;
        if is_empty {
            self.remove_element(parent);
        }
        Some(el)
    }
    #[inline]
    pub fn get_element(&self, id: Id) -> Option<&LayoutElement> {
        self.elements.get(&id)
    }
    #[inline]
    pub fn get_element_mut(&mut self, id: Id) -> Option<&mut LayoutElement> {
        self.elements.get_mut(&id)
    }
    #[inline]
    pub fn get_sublayout(&self, id: Id) -> Option<&SubLayout> {
        self.get_element(id).and_then(LayoutElement::get_sublayout)
    }
    #[inline]
    pub fn get_sublayout_mut(&mut self, id: Id) -> Option<&mut SubLayout> {
        self.get_element_mut(id)
            .and_then(LayoutElement::get_sublayout_mut)
    }
    #[inline]
    pub fn get_window(&self, id: Id) -> Option<&InnerWindow> {
        self.get_element(id)
            .and_then(LayoutElement::get_window)
            .map(|w| &w.inner)
    }
    #[inline]
    pub fn get_window_from_surface(&self, surface: &WlSurface) -> Option<&InnerWindow> {
        self.iter_windows()
            .find(|w| w.wl_surface().is_some_and(|s| s.as_ref() == surface))
    }
    pub fn insert_element(&mut self, parent: Id, element: LayoutElement) -> Option<Id> {
        self.insert_element_at(parent, Edge::empty(), element)
    }
    pub fn insert_element_at(
        &mut self,
        parent: Id,
        edge: Edge,
        mut element: LayoutElement,
    ) -> Option<Id> {
        let id = self.next_id;
        let el = self.get_element_mut(parent)?;
        match el {
            LayoutElement::SubLayout(sl) => {
                element.set_parent(parent);
                sl.children.push(id);
            }
            LayoutElement::Window(_) => {
                let parent_id = self.get_parent(parent).unwrap();
                let sl = self.get_sublayout_mut(parent_id).unwrap();
                element.set_parent(parent_id);
                let mut index = sl.children.iter().position(|i| i == &parent).unwrap();
                let edge = match sl.layout_type {
                    LayoutType::Horizontal => edge.get_horizontal(),
                    LayoutType::Vertical => edge.get_vertical(),
                    _ => Edge::empty(),
                };
                if !edge.intersects(Edge::TOP | Edge::LEFT) {
                    index += 1;
                }
                if index < sl.children.len() {
                    sl.children.insert(index, id);
                } else {
                    sl.children.push(id);
                }
            }
        }
        element.set_id(id);
        self.elements.insert(id, element);
        self.next_id = id.next();
        Some(id)
    }
    pub fn insert_sublayout(&mut self, parent: Id, layout_type: LayoutType) -> Option<Id> {
        self.insert_element(
            parent,
            LayoutElement::SubLayout(SubLayout {
                layout_type,
                id: self.next_id,
                ..Default::default()
            }),
        )
    }
    pub fn insert_window(&mut self, parent: Id, window: InnerWindow) -> Option<Id> {
        self.insert_element(
            parent,
            LayoutElement::Window(Window {
                id: self.next_id,
                parent: Default::default(),
                inner: window,
            }),
        )
    }
    #[inline]
    pub fn get_parent(&self, id: Id) -> Option<Id> {
        self.get_element(id).and_then(LayoutElement::parent)
    }
    #[inline]
    pub fn get_window_id(&self, window: &InnerWindow) -> Option<Id> {
        self.iter_pairs().find_map(|(i, e)| match e {
            LayoutElement::Window(w) if &w.inner == window => Some(i),
            _ => None,
        })
    }
    fn is_correct_layout_type(&self, layout_type: LayoutType, edge: Edge) -> bool {
        return (layout_type == LayoutType::Horizontal && edge.is_horizontal())
            || (layout_type == LayoutType::Vertical && edge.is_vertical());
    }
    pub fn get_window_neighbor(&self, id: Id, edge: Edge) -> Option<Id> {
        let mut sl = self.get_parent(id).and_then(|id| self.get_sublayout(id))?;
        let mut child_id = id;
        while !self.is_correct_layout_type(sl.layout_type, edge) {
            child_id = sl.id;
            sl = sl.parent.and_then(|id| self.get_sublayout(id))?;
        }
        let index = sl.children.iter().position(|e| e == &child_id)?;
        let new_index = match edge {
            Edge::TOP | Edge::LEFT => index.saturating_sub(1),
            Edge::BOTTOM | Edge::RIGHT => index + 1,
            _ => return None,
        };
        let mut id = sl.children.get(new_index).copied()?;
        while let Some(sl) = self.get_sublayout(id) {
            id = sl.children.get(sl.last_focused).copied()?;
        }
        Some(id)
    }
    pub fn on_focus(&mut self, id: Id) {
        let Some(parent) = self.get_parent(id) else {
            return;
        };
        self.get_sublayout_mut(parent).map(|sl| {
            sl.last_focused = sl
                .children
                .iter()
                .position(|i| i == &id)
                .unwrap_or_default();
        });
        self.on_focus(parent);
    }
}

impl PocoWM {
    pub fn switch_to_layout(&mut self, layout_type: LayoutType) -> Option<()> {
        let focused_window = self.seat.get_keyboard().and_then(|k| k.current_focus());
        if let Some(focused_window) = focused_window {
            if focused_window.state().contains(WindowState::FLOATING) {
                return None;
            }
            let id = self.layout.get_window_id(&focused_window)?;
            let parent = self.layout.insert_sublayout(id, layout_type)?;
            let el = self.layout.remove_element(id)?;
            self.layout.insert_element(parent, el);
        } else {
            self.layout.root_mut().layout_type = layout_type;
        }
        Some(())
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
