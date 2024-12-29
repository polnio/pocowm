use std::cell::RefCell;

use derive_more::{Deref, DerefMut, IsVariant};
use getset::{Getters, Setters};
use smithay::desktop::Window as InnerWindow;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
use smithay::utils::{Logical, Point, Rectangle, Size};
use smithay::wayland::shell::xdg::ToplevelSurface;

use crate::renderer::decorations::DECORATIONS_SIZE;

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
    floating_rect: Option<Rectangle<i32, Logical>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Getters, Setters, Deref, DerefMut)]
#[getset(get = "pub", set = "pub")]
pub struct Window {
    #[deref]
    #[deref_mut]
    inner: InnerWindow,
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
    pub fn resize(&self, mut size: Size<i32, Logical>) {
        if self.has_decorations() {
            size.h -= DECORATIONS_SIZE as i32;
        }
        let Some(xdg) = self.inner.toplevel() else {
            return;
        };
        xdg.with_pending_state(|state| {
            state.size = Some(size);
        });
        xdg.send_configure();
    }
    pub fn floating_rect(&self) -> Option<Rectangle<i32, Logical>> {
        self.user_data().borrow().floating_rect.clone()
    }
    pub fn floating_rect_or_default(&self) -> Rectangle<i32, Logical> {
        let mut data = self.user_data().borrow_mut();
        if data.floating_rect.is_none() {
            // TODO: Calculate floating rect
            let rect = Rectangle::from_loc_and_size(
                (20, 20),
                // self.geometry().size + Size::from((DECORATIONS_SIZE as i32, DECORATIONS_SIZE as i32)),
                self.geometry().size,
            );
            data.floating_rect = Some(rect);
        }
        data.floating_rect.unwrap()
    }
    pub fn set_floating_rect(&self, rect: Rectangle<i32, Logical>) {
        self.user_data().borrow_mut().floating_rect = Some(rect);
        // self.user_data().borrow_mut().floating_rect = rect;
        // self.floating_rect = rect;
    }
    pub fn floating_loc(&self) -> Point<i32, Logical> {
        self.floating_rect_or_default().loc
    }
    pub fn set_floating_loc(&self, loc: Point<i32, Logical>) {
        self.floating_rect_or_default();
        self.user_data()
            .borrow_mut()
            .floating_rect
            .as_mut()
            .unwrap()
            .loc = loc;
        // self.user_data().borrow_mut().floating_rect.loc = loc;
        // self.floating_rect.loc = loc;
    }
    pub fn floating_size(&self) -> Size<i32, Logical> {
        self.floating_rect_or_default().size
        // self.user_data().borrow().floating_rect.size
        // self.floating_rect.size
    }
    pub fn set_floating_size(&self, size: Size<i32, Logical>) {
        self.floating_rect_or_default();
        self.user_data()
            .borrow_mut()
            .floating_rect
            .as_mut()
            .unwrap()
            .size = size;
        // self.user_data().borrow_mut().floating_rect.size = size;
        // self.floating_rect.size = size;
    }
    pub fn state(&self) -> WindowState {
        self.user_data().borrow().state.clone()
    }
    pub fn set_state(&self, state: WindowState) {
        self.user_data().borrow_mut().state = state;
    }
    pub fn has_decorations(&self) -> bool {
        self.toplevel().is_none_or(|t| {
            t.with_pending_state(|state| {
                let mode = Some(Mode::ServerSide);
                state.decoration_mode == mode
            })
        })
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
