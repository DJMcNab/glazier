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

#![allow(clippy::single_match)]

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{
        mpsc::{Sender, TryRecvError},
        Arc, Mutex,
    },
};

use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::{
        calloop::{EventLoop, LoopHandle, LoopSignal},
        client::{
            globals::registry_queue_init, protocol::wl_compositor, Connection, QueueHandle,
            WaylandSource,
        },
    },
    registry::RegistryState,
    shell::xdg::XdgShell,
};

use super::{clipboard, error::Error, IdleCallback, WaylandState};
use crate::{backend::shared::linux, AppHandler};

#[derive(Clone)]
pub struct Application {
    // `State` is the items stored between `new` and `run`
    // It is stored in an Rc<RefCell>> because Application must be Clone
    // These items are `take`n in run
    state: Rc<RefCell<Option<WaylandState>>>,
    compositor: wl_compositor::WlCompositor,
    wayland_queue: QueueHandle<WaylandState>,
    loop_handle: LoopHandle<'static, WaylandState>,
    loop_signal: LoopSignal,
    // This Mutex is only required as Sender is (currently artificially) not Sync
    idle_sender: Arc<Mutex<Sender<IdleCallback>>>,
}

impl Application {
    pub fn new() -> Result<Self, Error> {
        tracing::info!("wayland application initiated");

        let conn = Connection::connect_to_env()?;
        let (globals, event_queue) = registry_queue_init::<WaylandState>(&conn).unwrap();
        let qh = event_queue.handle();
        let event_loop: EventLoop<WaylandState> = EventLoop::try_new()?;
        let loop_handle = event_loop.handle();
        let loop_signal = event_loop.get_signal();

        WaylandSource::new(event_queue)
            .unwrap()
            .insert(loop_handle.clone())
            .unwrap();

        let compositor_state: CompositorState = CompositorState::bind(&globals, &qh)?;
        let compositor = compositor_state.wl_compositor().clone();

        let (idle_sender, idle_callbacks) = std::sync::mpsc::channel();
        let idle_sender = Arc::new(Mutex::new(idle_sender));
        let state = WaylandState {
            registry_state: RegistryState::new(&globals),
            output_state: OutputState::new(&globals, &qh),
            compositor_state,
            xdg_shell_state: XdgShell::bind(&globals, &qh)?,
            event_loop: Some(event_loop),
            handler: None,
            idle_callbacks,
            idle_sender: idle_sender.clone(),
            windows: HashMap::new(),
        };
        Ok(Application {
            state: Rc::new(RefCell::new(Some(state))),
            compositor,
            wayland_queue: qh,
            loop_handle,
            loop_signal,
            idle_sender,
        })
    }

    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        tracing::info!("wayland event loop initiated");
        let mut state = self
            .state
            .borrow_mut()
            .take()
            .expect("Can only run an application once");
        let mut event_loop = state.event_loop.take().unwrap();
        event_loop
            .run(None, &mut state, |state| loop {
                match state.idle_callbacks.try_recv() {
                    Ok(cb) => cb(state),
                    Err(TryRecvError::Empty) => (),
                    Err(TryRecvError::Disconnected) => {
                        unreachable!("Backend has allowed the shared Sender to be dropped")
                    }
                }
            })
            .expect("Shouldn't error in event loop");
    }

    pub fn quit(&self) {
        // Stopping the event loop serves to
        self.loop_signal.stop();
    }

    pub fn clipboard(&self) -> clipboard::Clipboard {
        // TODO: Wayland's clipboard is inherently asynchronous (as is the web)
        clipboard::Clipboard {}
    }

    pub fn get_locale() -> String {
        linux::env::locale()
    }

    pub fn get_handle(&self) -> Option<AppHandle> {
        Some(AppHandle {
            wayland_queue: self.wayland_queue.clone(),
            loop_signal: self.loop_signal.clone(),
            idle_sender: self.idle_sender.clone(),
        })
    }
}

#[derive(Clone)]
pub struct AppHandle {
    wayland_queue: QueueHandle<WaylandState>,
    loop_signal: LoopSignal,
    idle_sender: Arc<Mutex<Sender<IdleCallback>>>,
}

impl AppHandle {
    pub fn run_on_main<F>(&self, callback: F)
    where
        F: FnOnce(Option<&mut dyn AppHandler>) + Send + 'static,
    {
        // For reasons unknown, inlining this call gives lifetime errors
        // Luckily, this appears to work, so just leave it there
        self.run_on_main_inner(|it| {
            callback(match it {
                Some(it) => Some(&mut **it),
                None => None,
            })
        })
    }

    pub fn run_on_main_inner<F>(&self, callback: F)
    where
        F: FnOnce(Option<&mut Box<dyn AppHandler>>) + Send + 'static,
    {
        self.idle_sender
            .lock()
            .unwrap()
            .send(Box::new(move |state| callback(state.handler.as_mut())))
            .expect("AppHandle should exist whilst");
        self.loop_signal.wakeup();
    }
}
