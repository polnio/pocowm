use bitflags::bitflags;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;

bitflags! {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct Edge: u32 {
        const TOP    = 0b0001;
        const BOTTOM = 0b0010;
        const LEFT   = 0b0100;
        const RIGHT  = 0b1000;
    }
}
impl From<xdg_toplevel::ResizeEdge> for Edge {
    fn from(edge: xdg_toplevel::ResizeEdge) -> Self {
        match edge {
            xdg_toplevel::ResizeEdge::Top => Self::TOP,
            xdg_toplevel::ResizeEdge::Bottom => Self::BOTTOM,
            xdg_toplevel::ResizeEdge::Left => Self::LEFT,
            xdg_toplevel::ResizeEdge::Right => Self::RIGHT,
            xdg_toplevel::ResizeEdge::TopLeft => Self::TOP | Self::LEFT,
            xdg_toplevel::ResizeEdge::BottomLeft => Self::BOTTOM | Self::LEFT,
            xdg_toplevel::ResizeEdge::TopRight => Self::TOP | Self::RIGHT,
            xdg_toplevel::ResizeEdge::BottomRight => Self::BOTTOM | Self::RIGHT,
            _ => Self::empty(),
        }
    }
}
impl Edge {
    pub fn get_vertical(&self) -> Edge {
        self.intersection(Self::TOP | Self::BOTTOM)
    }
    pub fn get_horizontal(&self) -> Edge {
        self.intersection(Self::LEFT | Self::RIGHT)
    }
    pub fn is_horizontal(&self) -> bool {
        !self.get_horizontal().is_empty()
    }
    pub fn is_vertical(&self) -> bool {
        !self.get_vertical().is_empty()
    }
}
