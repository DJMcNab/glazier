use smithay_client_toolkit::{delegate_pointer, seat::pointer::PointerHandler};

use crate::backend::wayland::WaylandState;

// Ideally, we would use sctk's pointer handling, but there are a few issues:
// 1) It doesn't allow us to store our own seat identifier
// 2) It doesn't implement the latest methods
// 3) It uses Mutexes in places we wouldn't need it to
// (not blocking us, but interesting)

impl PointerHandler for WaylandState {
    fn pointer_frame(
        &mut self,
        conn: &smithay_client_toolkit::reexports::client::Connection,
        qh: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
        pointer: &smithay_client_toolkit::reexports::client::protocol::wl_pointer::WlPointer,
        events: &[smithay_client_toolkit::seat::pointer::PointerEvent],
    ) {
        todo!()
    }
}

delegate_pointer!(WaylandState);
