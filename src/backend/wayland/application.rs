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

use super::{clipboard, error::Error};
use crate::{backend::shared::linux, AppHandler};

#[derive(Clone)]
pub struct Application {}

impl Application {
    pub fn new() -> Result<Self, Error> {
        tracing::info!("wayland application initiated");

        Ok(Application {})
    }

    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        tracing::info!("wayland event loop initiated");
        todo!()
    }

    pub fn quit(&self) {
        todo!()
    }

    pub fn clipboard(&self) -> clipboard::Clipboard {
        todo!()
    }

    pub fn get_locale() -> String {
        linux::env::locale()
    }

    pub fn get_handle(&self) -> Option<AppHandle> {
        None
    }
}
#[derive(Clone)]
pub struct AppHandle;

impl AppHandle {
    pub fn run_on_main<F>(&self, _callback: F)
    where
        F: FnOnce(Option<&mut dyn AppHandler>) + Send + 'static,
    {
        todo!()
    }
}
