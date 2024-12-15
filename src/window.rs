use std::cell::RefCell;

use derive_more::{Deref, DerefMut, IsVariant};
use getset::{Getters, Setters};
use smithay::desktop::Window as InnerWindow;
use smithay::utils::{Logical, Point, Rectangle, Size};
use smithay::wayland::shell::xdg::ToplevelSurface;

#[derive(Debug, Clone, Default, PartialEq, Eq, IsVariant)]
pub enum WindowState {
    #[default]
    Tiled,
    Floating,
    Maximized,
    Fullscreen,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct WindowUserData {
    state: WindowState,
    floating_rect: Rectangle<i32, Logical>,
}

#[derive(Debug, Clone, PartialEq, Eq, Getters, Setters, Deref, DerefMut)]
#[getset(get = "pub", set = "pub")]
pub struct Window {
    #[deref]
    #[deref_mut]
    inner: InnerWindow,
    // state: WindowState,
    // floating_rect: Rectangle<i32, Logical>,
}

impl Window {
    pub fn from_surface(surface: ToplevelSurface) -> Self {
        Self::from(InnerWindow::new_wayland_window(surface))
    }
    fn user_data(&self) -> &RefCell<WindowUserData> {
        self.inner
            .user_data()
            .get_or_insert(|| RefCell::new(WindowUserData::default()))
    }
    pub fn floating_rect(&self) -> Rectangle<i32, Logical> {
        self.user_data().borrow().floating_rect
    }
    pub fn set_floating_rect(&mut self, rect: Rectangle<i32, Logical>) {
        self.user_data().borrow_mut().floating_rect = rect;
        // self.floating_rect = rect;
    }
    pub fn floating_loc(&self) -> Point<i32, Logical> {
        self.user_data().borrow().floating_rect.loc
    }
    pub fn set_floating_loc(&mut self, loc: Point<i32, Logical>) {
        self.user_data().borrow_mut().floating_rect.loc = loc;
        // self.floating_rect.loc = loc;
    }
    pub fn floating_size(&self) -> Size<i32, Logical> {
        self.user_data().borrow().floating_rect.size
        // self.floating_rect.size
    }
    pub fn set_floating_size(&mut self, size: Size<i32, Logical>) {
        self.user_data().borrow_mut().floating_rect.size = size;
        // self.floating_rect.size = size;
    }
    pub fn state(&self) -> WindowState {
        self.user_data().borrow().state.clone()
    }
    pub fn set_state(&mut self, state: WindowState) {
        self.user_data().borrow_mut().state = state;
    }
}

impl From<InnerWindow> for Window {
    fn from(inner: InnerWindow) -> Self {
        Self {
            inner,
            // state: Default::default(),
            // floating_rect: Default::default(),
        }
    }
}

impl Into<InnerWindow> for Window {
    fn into(self) -> InnerWindow {
        self.inner
    }
}
