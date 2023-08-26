use smithay_client_toolkit::{
    delegate_pointer,
    reexports::client::{
        protocol::{
            wl_pointer::{self, WlPointer},
            wl_seat,
        },
        Dispatch, QueueHandle,
    },
    seat::pointer::PointerHandler,
};

use crate::backend::wayland::WaylandState;

use super::{input_state, SeatInfo, SeatName};

// Ideally, we would use sctk's pointer handling, but there are a few issues:
// 1) It doesn't allow us to store our own seat identifier
// 2) It doesn't implement the latest methods
// 3) It uses Mutexes in places we wouldn't need it to
// (not blocking us, but interesting)

struct Pointer(());

impl Dispatch<WlPointer, PointerUserData> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &WlPointer,
        event: <WlPointer as smithay_client_toolkit::reexports::client::Proxy>::Event,
        data: &PointerUserData,
        conn: &smithay_client_toolkit::reexports::client::Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                serial,
                surface,
                surface_x,
                surface_y,
            } => {
                todo!("Call handler::pointer_move (TODO: Why no pointer_enter?), then update the cursor to the provided cursor of this window (in `frame`)");
            }
            wl_pointer::Event::Leave { serial, surface } => {
                todo!("Call handler::pointer_leave (in `frame`)");
            }
            wl_pointer::Event::Motion {
                time,
                surface_x,
                surface_y,
            } => {
                todo!("Call handler::pointer_move (in `frame`)");
            }
            wl_pointer::Event::Button {
                serial,
                time,
                button,
                state,
            } => todo!("Call handler::pointer_down or pointer_up (in `frame`). Don't forget to debounce double (/triple?) clicks"),
            wl_pointer::Event::Axis { time, axis, value } => todo!("Call handler::wheel (in `frame`). Note that this API doesn't exist yet"),
            wl_pointer::Event::AxisSource { axis_source } => todo!("We need to work out exact semantics around kinetic scrolling with fingers"),
            wl_pointer::Event::AxisStop { time, axis } => todo!("Accumulate result"),
            wl_pointer::Event::AxisDiscrete { axis, discrete } => todo!(),
            wl_pointer::Event::AxisValue120 { axis, value120 } => todo!(),
            wl_pointer::Event::AxisRelativeDirection { axis, direction } => todo!(),

            wl_pointer::Event::Frame => todo!(),
            _ => todo!(),
        }
    }
}

/// The seat identifier of this keyboard
struct PointerUserData(SeatName);

pub(super) struct PointerState {
    pointer: WlPointer,
}

fn pointer<'a>(seats: &'a mut [SeatInfo], data: &PointerUserData) -> &'a mut PointerState {
    input_state(seats, data.0).pointer_state.as_mut().expect(
        "KeyboardUserData is only constructed when a new keyboard is created, so state exists",
    )
}

impl Drop for PointerState {
    fn drop(&mut self) {
        self.pointer.release()
    }
}

impl PointerState {
    pub(super) fn new(
        qh: &QueueHandle<WaylandState>,
        name: SeatName,
        seat: wl_seat::WlSeat,
    ) -> Self {
        PointerState {
            pointer: seat.get_pointer(qh, PointerUserData(name)),
        }
    }
}
