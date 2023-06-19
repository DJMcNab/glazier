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

use std::borrow::Borrow;
use std::marker::PhantomData;

use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use smithay_client_toolkit::compositor::CompositorHandler;
use smithay_client_toolkit::reexports::client::{protocol, Connection, QueueHandle};
use smithay_client_toolkit::shell::xdg::window::{DecorationMode, Window, WindowHandler};
use smithay_client_toolkit::shell::xdg::{XdgShellSurface, XdgSurface};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::{delegate_compositor, delegate_xdg_shell, delegate_xdg_window};
use tracing;
use wayland_backend::client::ObjectId;

use super::application::{self};
use super::menu::Menu;
use super::WaylandState;

use crate::IdleToken;
use crate::{
    dialog::FileDialogOptions,
    error::Error as ShellError,
    kurbo::{Insets, Point, Rect, Size},
    mouse::{Cursor, CursorDesc},
    scale::Scale,
    text::Event,
    window::{self, FileDialogToken, TimerToken, WinHandler, WindowLevel},
    TextFieldToken,
};

#[derive(Clone)]
pub struct WindowHandle {
    wayland_window: Window,
    not_send: PhantomData<*mut ()>,
}

impl WindowHandle {
    pub fn id(&self) -> u64 {
        todo!()
    }

    pub fn show(&self) {
        tracing::debug!("show initiated");
        self.wayland_window.offset(x, y)
    }

    pub fn resizable(&self, resizable: bool) {
        tracing::warn!("resizable is unimplemented on wayland");
        // TODO: If we are using fallback decorations, we should be able to disable
        // dragging based resizing
    }

    pub fn show_titlebar(&self, show_titlebar: bool) {
        tracing::warn!("show_titlebar is implemented on a best-effort basis on wayland");
        if show_titlebar {
            self.wayland_window
                .request_decoration_mode(Some(DecorationMode::Server))
        } else {
            // TODO: Track this into the fallback decorations, somehow
            self.wayland_window
                .request_decoration_mode(Some(DecorationMode::Client))
        }
    }

    pub fn set_position(&self, _position: Point) {
        tracing::warn!("set_position is unimplemented on wayland");
        // TODO: Use the KDE plasma extensions for this if available
        // TODO: Use xdg_positioner if this is a child window
    }

    pub fn get_position(&self) -> Point {
        tracing::warn!("get_position is unimplemented on wayland");
        Point::ZERO
    }

    pub fn content_insets(&self) -> Insets {
        // I *think* wayland surfaces don't care about content insets
        // That is, all decorations are 'outsets'. Therefore this is complete
        Insets::from(0.)
    }

    pub fn set_size(&self, _size: Size) {
        tracing::warn!("set_size is unimplemented on wayland");
    }

    pub fn get_size(&self) -> Size {
        // I think we need to cache the size ourselves
        todo!()
    }

    pub fn set_window_state(&mut self, state: window::WindowState) {
        match state {
            crate::WindowState::Maximized => self.wayland_window.set_maximized(),
            crate::WindowState::Minimized => self.wayland_window.set_minimized(),
            // TODO: I don't think we can do much better than this - we can't unset being minimised
            crate::WindowState::Restored => self.wayland_window.unset_maximized(),
        }
    }

    pub fn get_window_state(&self) -> window::WindowState {
        // We can know if we're maximised
        tracing::warn!("get_window_state is unimplemented on wayland");
        window::WindowState::Maximized
    }

    pub fn handle_titlebar(&self, _val: bool) {
        tracing::warn!("handle_titlebar is unimplemented on wayland");
    }

    /// Close the window.
    pub fn close(&self) {
        todo!()
    }

    /// Bring this window to the front of the window stack and give it focus.
    pub fn bring_to_front_and_focus(&self) {
        tracing::warn!("unimplemented bring_to_front_and_focus initiated");
    }

    /// Request a new paint, but without invalidating anything.
    pub fn request_anim_frame(&self) {
        todo!()
    }

    /// Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        todo!()
    }

    /// Request invalidation of one rectangle, which is given in display points relative to the
    /// drawing area.
    pub fn invalidate_rect(&self, _rect: Rect) {
        todo!()
    }

    pub fn add_text_field(&self) -> TextFieldToken {
        todo!()
    }

    pub fn remove_text_field(&self, _token: TextFieldToken) {
        todo!()
    }

    pub fn set_focused_text_field(&self, _active_field: Option<TextFieldToken>) {
        todo!()
    }

    pub fn update_text_field(&self, _token: TextFieldToken, _update: Event) {
        // noop until we get a real text input implementation
    }

    pub fn request_timer(&self, _deadline: std::time::Instant) -> TimerToken {
        todo!()
    }

    pub fn set_cursor(&mut self, _cursor: &Cursor) {
        todo!()
    }

    pub fn make_cursor(&self, _desc: &CursorDesc) -> Option<Cursor> {
        tracing::warn!("unimplemented make_cursor initiated");
        None
    }

    pub fn open_file(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken> {
        tracing::warn!("unimplemented open_file");
        None
    }

    pub fn save_as(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken> {
        tracing::warn!("unimplemented save_as");
        None
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        todo!()
    }

    /// Get the `Scale` of the window.
    pub fn get_scale(&self) -> Result<Scale, ShellError> {
        todo!()
    }

    pub fn set_menu(&self, _menu: Menu) {
        tracing::warn!("set_menu not implement for wayland");
    }

    pub fn show_context_menu(&self, _menu: Menu, _pos: Point) {
        tracing::warn!("show_context_menu not implement for wayland");
    }

    pub fn set_title(&self, _title: impl Into<String>) {
        todo!()
    }

    #[cfg(feature = "accesskit")]
    pub fn update_accesskit_if_active(
        &self,
        _update_factory: impl FnOnce() -> accesskit::TreeUpdate,
    ) {
        // AccessKit doesn't yet support this backend.
    }
}

impl PartialEq for WindowHandle {
    fn eq(&self, _rhs: &Self) -> bool {
        todo!()
    }
}

impl Eq for WindowHandle {}

impl Default for WindowHandle {
    fn default() -> WindowHandle {
        // TODO: Why is this Default?
        WindowHandle {
            not_send: Default::default(),
        }
    }
}

unsafe impl HasRawWindowHandle for WindowHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        tracing::error!("HasRawWindowHandle trait not implemented for wasm.");
        todo!()
    }
}

unsafe impl HasRawDisplayHandle for WindowHandle {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        tracing::error!("HasDisplayHandle trait not implemented for wayland.");
        todo!()
    }
}

#[derive(Clone)]
pub struct IdleHandle {}

impl IdleHandle {
    pub fn add_idle_callback<F>(&self, _callback: F)
    where
        F: FnOnce(&mut dyn WinHandler) + Send + 'static,
    {
        todo!();
    }

    pub fn add_idle_token(&self, _token: IdleToken) {
        todo!();
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CustomCursor;

/// Builder abstraction for creating new windows
pub(crate) struct WindowBuilder {
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    menu: Option<Menu>,
    position: Option<Point>,
    level: WindowLevel,
    state: Option<window::WindowState>,
    // pre-scaled
    size: Size,
    min_size: Option<Size>,
    resizable: bool,
    show_titlebar: bool,
}

impl WindowBuilder {
    pub fn new(_app: application::Application) -> WindowBuilder {
        WindowBuilder {
            handler: None,
            title: String::new(),
            menu: None,
            size: Size::new(0.0, 0.0),
            position: None,
            level: WindowLevel::AppWindow,
            state: None,
            min_size: None,
            resizable: true,
            show_titlebar: true,
        }
    }

    pub fn handler(mut self, handler: Box<dyn WinHandler>) -> Self {
        self.handler = Some(handler);
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn min_size(mut self, size: Size) -> Self {
        self.min_size = Some(size);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn show_titlebar(mut self, show_titlebar: bool) -> Self {
        self.show_titlebar = show_titlebar;
        self
    }

    pub fn transparent(self, _transparent: bool) -> Self {
        tracing::warn!(
            "WindowBuilder::transparent is unimplemented for Wayland, it allows transparency by default"
        );
        self
    }

    pub fn position(mut self, position: Point) -> Self {
        self.position = Some(position);
        self
    }

    pub fn level(mut self, level: WindowLevel) -> Self {
        self.level = level;
        self
    }

    pub fn window_state(mut self, state: window::WindowState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn menu(mut self, menu: Menu) -> Self {
        self.menu = Some(menu);
        self
    }

    pub fn build(self) -> Result<WindowHandle, ShellError> {
        todo!()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct WindowId(ObjectId);

impl WindowId {
    pub fn new(surface: &impl WaylandSurface) -> Self {
        Self(surface.wl_surface().borrow().clone())
    }
}

/// The state associated with each window, stored in [`WaylandState`]
pub struct WindowState {
    handler: Box<dyn WinHandler>,
    wayland_window: Window,
}

delegate_xdg_shell!(WaylandState);
delegate_xdg_window!(WaylandState);

delegate_compositor!(WaylandState);

impl CompositorHandler for WaylandState {
    fn scale_factor_changed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &protocol::wl_surface::WlSurface,
        new_factor: i32,
    ) {
        let window = self.windows.get_mut(&WindowId::new(surface));
    }

    fn frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &protocol::wl_surface::WlSurface,
        time: u32,
    ) {
        let window = self.windows.get_mut(&WindowId::new(surface));
    }
}

impl WindowHandler for WaylandState {
    fn request_close(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        window: &smithay_client_toolkit::shell::xdg::window::Window,
    ) {
        todo!()
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        window: &smithay_client_toolkit::shell::xdg::window::Window,
        configure: smithay_client_toolkit::shell::xdg::window::WindowConfigure,
        serial: u32,
    ) {
        window.show_window_menu(seat, serial, position)
    }
}
