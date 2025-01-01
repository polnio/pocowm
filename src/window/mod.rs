pub mod borders;
pub mod decorations;
pub mod render;
pub mod seat;

use borders::Borders;
use decorations::{Decorations, DECORATIONS_HEIGHT};
use derive_more::{Deref, DerefMut, IsVariant};
use getset::{Getters, Setters};
use smithay::desktop::Window as InnerWindow;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
use smithay::utils::{Logical, Point, Rectangle, Size};
use smithay::wayland::shell::xdg::ToplevelSurface;
use std::cell::{Ref, RefCell, RefMut};

#[derive(Debug, Clone, Default, PartialEq, Eq, IsVariant)]
pub enum WindowState {
    #[default]
    Tiled,
    Floating,
    Maximized,
    Fullscreen,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WindowSeatData {
    pub pointer_location: Option<Point<f64, Logical>>,
    pub touch_location: Option<Point<f64, Logical>>,
}

#[derive(Debug, Clone, PartialEq, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
struct WindowUserData {
    state: WindowState,
    is_focused: bool,
    floating_rect: Rectangle<i32, Logical>,
    seat_data: WindowSeatData,
    decorations: Option<Decorations>,
    borders: Borders,
}

impl WindowUserData {
    pub fn new(window: Window) -> Self {
        Self {
            state: Default::default(),
            is_focused: Default::default(),
            floating_rect: Default::default(),
            seat_data: Default::default(),
            decorations: Default::default(),
            borders: Borders::new(window),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
pub struct Window(InnerWindow);

macro_rules! generate_getter {
    ($vis:vis $field:ident: $ty:ty) => {
        generate_getter!($vis $field as $field: $ty);
    };
    ($vis:vis $field:ident as $alias:ident: $ty:ty) => {
        $vis fn $alias(&self) -> Ref<$ty> {
            Ref::map(self.user_data().borrow(), |data| &data.$field)
        }
        paste::paste! {
            pub fn [<$alias _mut>](&self) -> RefMut<$ty> {
                RefMut::map(self.user_data().borrow_mut(), |data| &mut data.$field)
            }
        }
    };
}

impl Window {
    #[inline]
    pub fn from_surface(surface: ToplevelSurface) -> Self {
        Self::from(InnerWindow::new_wayland_window(surface))
    }
    #[inline]
    pub fn inner(&self) -> &InnerWindow {
        &self.0
    }
    fn user_data(&self) -> &RefCell<WindowUserData> {
        self.inner()
            .user_data()
            .get_or_insert(|| RefCell::new(WindowUserData::new(self.clone())))
    }

    pub fn add_decorations(&self) {
        if let Some(xdg) = self.toplevel() {
            xdg.with_pending_state(|state| {
                state.decoration_mode = Some(Mode::ServerSide);
            });
            if xdg.is_initial_configure_sent() {
                xdg.send_pending_configure();
            }
        }
        let mut decorations = self.decorations_mut();
        if decorations.is_none() {
            *decorations = Some(Decorations::new(self.clone()));
        }
    }

    pub fn remove_decorations(&self) {
        if let Some(xdg) = self.toplevel() {
            xdg.with_pending_state(|state| {
                state.decoration_mode = Some(Mode::ClientSide);
            });
        }
        *self.decorations_mut() = None;
    }

    pub fn resize(&self, mut size: Size<i32, Logical>) {
        if self.decorations().is_some() {
            size.h -= DECORATIONS_HEIGHT as i32;
        }
        let Some(xdg) = self.0.toplevel() else {
            return;
        };
        xdg.with_pending_state(|state| {
            state.size = Some(size);
        });
        xdg.send_configure();
    }

    pub fn is_focused(&self) -> bool {
        *self.get_is_focused()
    }
    pub fn focus(&self) {
        self.set_activated(true);
        *self.get_is_focused_mut() = true;
    }
    pub fn unfocus(&self) {
        self.set_activated(false);
        *self.get_is_focused_mut() = false;
    }

    generate_getter!(pub state: WindowState);
    generate_getter!(pub floating_rect: Rectangle<i32, Logical>);
    generate_getter!(pub seat_data: WindowSeatData);
    generate_getter!(is_focused as get_is_focused: bool);
    generate_getter!(decorations: Option<Decorations>);
    generate_getter!(borders: Borders);
}

impl From<InnerWindow> for Window {
    #[inline]
    fn from(inner: InnerWindow) -> Self {
        Self(inner)
    }
}

impl Into<InnerWindow> for Window {
    #[inline]
    fn into(self) -> InnerWindow {
        self.0
    }
}
