// Copyright 2019 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! wayland platform support

use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};

use smithay_client_toolkit::{
    compositor::CompositorState,
    delegate_registry,
    output::OutputState,
    reexports::calloop::EventLoop,
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::xdg::XdgShell,
};

use crate::AppHandler;

pub mod application;
pub mod clipboard;
pub mod error;
pub mod menu;
pub mod screen;
pub mod window;

struct WaylandState {
    pub registry_state: RegistryState,
    // seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShell,

    pub event_loop: Option<EventLoop<'static, Self>>,
    pub handler: Option<Box<dyn AppHandler>>,
    pub idle_callbacks: Receiver<IdleCallback>,
    pub idle_sender: Arc<Mutex<Sender<IdleCallback>>>,
}

delegate_registry!(WaylandState);

impl ProvidesRegistryState for WaylandState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

type IdleCallback = Box<dyn FnOnce(&mut WaylandState) + Send>;
