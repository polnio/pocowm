use crate::PocoWM;
use smithay::delegate_data_device;
use smithay::wayland::selection::data_device::{
    ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
};
use smithay::wayland::selection::SelectionHandler;

impl SelectionHandler for PocoWM {
    type SelectionUserData = ();
}
impl ClientDndGrabHandler for PocoWM {}
impl ServerDndGrabHandler for PocoWM {}

impl DataDeviceHandler for PocoWM {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}
delegate_data_device!(PocoWM);
