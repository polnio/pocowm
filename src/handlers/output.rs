use crate::PocoWM;
use smithay::delegate_output;
use smithay::wayland::output::OutputHandler;

impl OutputHandler for PocoWM {}

delegate_output!(PocoWM);
