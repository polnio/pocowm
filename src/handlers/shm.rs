use crate::PocoWM;
use smithay::delegate_shm;
use smithay::wayland::shm::ShmHandler;

impl ShmHandler for PocoWM {
    fn shm_state(&self) -> &smithay::wayland::shm::ShmState {
        &self.shm_state
    }
}

delegate_shm!(PocoWM);
