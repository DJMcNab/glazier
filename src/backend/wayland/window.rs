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

use std::borrow::BorrowMut;
use std::marker::PhantomData;
use std::os::raw::c_void;
use std::sync::{Arc, RwLock, Weak};

use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
    WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::compositor::CompositorHandler;
use smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface;
use smithay_client_toolkit::reexports::client::{protocol, Connection, Proxy, QueueHandle};
use smithay_client_toolkit::shell::xdg::window::{DecorationMode, Window, WindowHandler};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::{delegate_compositor, delegate_xdg_shell, delegate_xdg_window};
use tracing;
use wayland_backend::client::ObjectId;

use super::application::{self, AppHandle};
use super::menu::Menu;
use super::WaylandState;

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
use crate::{IdleToken, Region, Scalable};

#[derive(Clone)]
pub struct WindowHandle {
    // Annoyingly Option, because we must be default
    wayland_window: Option<Window>,
    app: Option<AppHandle>,
    properties: Weak<RwLock<WindowProperties>>,
    // Safety: Points to a wl_display instance
    raw_display_handle: Option<*mut c_void>,
    not_send: PhantomData<*mut ()>,
}

impl WindowHandle {
    #[track_caller]
    /// Get the wayland window underlying this real window
    /// This method unwraps the inner window (which may be None because we
    /// have to implement `Default`)
    fn wayland_window(&self) -> &Window {
        self.wayland_window
            .as_ref()
            .expect("Called operation on dead window")
    }

    fn properties(&self) -> Arc<RwLock<WindowProperties>> {
        self.properties.upgrade().unwrap()
    }

    pub fn show(&self) {
        tracing::debug!("show initiated");
        self.wayland_window().commit();
        self.request_anim_frame();
    }

    pub fn resizable(&self, resizable: bool) {
        tracing::warn!("resizable is unimplemented on wayland");
        // TODO: If we are using fallback decorations, we should be able to disable
        // dragging based resizing
    }

    pub fn show_titlebar(&self, show_titlebar: bool) {
        tracing::warn!("show_titlebar is implemented on a best-effort basis on wayland");
        // TODO: Track this into the fallback decorations when we add those
        if show_titlebar {
            self.wayland_window()
                .request_decoration_mode(Some(DecorationMode::Server))
        } else {
            self.wayland_window()
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
        // That is, all decorations (to confirm: even client side?) are 'outsets'
        Insets::from(0.)
    }

    pub fn set_size(&self, size: Size) {
        {
            let props = self.properties();
            let mut props = props.write().unwrap();
            props.requested_size = Some(size);
        }

        let window_id = WindowId::new(self.wayland_window());
        // We don't need to tell the server about changing the size - so long as the size of the surface gets changed properly
        // So, all we need to do is to tell the handler about this change (after caching it here)

        // We must defer this, because we're probably in the handler, which we need to call
        self.app.as_ref().unwrap().run_on_state(move |state| {
            let window = {
                let Some(window) = state.windows.get_mut(&window_id) else { return };

                let mut props = window.properties.write().unwrap();
                let size = props.requested_size.expect("Can't unset requested size");
                props.current_size = size;
                window.handler.size(size);
                window.wayland_window.clone()
            };
            let surface = window.wl_surface();
            // Request a redraw now that the size has changed
            surface.frame(&state.wayland_queue.clone(), surface.clone());
        });
    }

    pub fn get_size(&self) -> Size {
        let props = self.properties();
        let props = props.read().unwrap();
        props.current_size
    }

    pub fn set_window_state(&mut self, state: window::WindowState) {
        match state {
            crate::WindowState::Maximized => self.wayland_window().set_maximized(),
            crate::WindowState::Minimized => self.wayland_window().set_minimized(),
            // TODO: I don't think we can do much better than this - we can't unset being minimised
            crate::WindowState::Restored => self.wayland_window().unset_maximized(),
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
        Some(IdleHandle {
            app: self.app.as_ref().unwrap().clone(),
            window: WindowId::new(self.wayland_window()),
        })
    }

    /// Get the `Scale` of the window.
    pub fn get_scale(&self) -> Result<Scale, ShellError> {
        let props = self.properties();
        let props = props.read().unwrap();
        Ok(props.current_scale)
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
    fn eq(&self, rhs: &Self) -> bool {
        self.wayland_window() == rhs.wayland_window()
    }
}

impl Eq for WindowHandle {}

impl Default for WindowHandle {
    fn default() -> WindowHandle {
        // TODO: Why is this Default?
        WindowHandle {
            not_send: Default::default(),
            wayland_window: None,
            app: None,
            properties: Weak::new(),
            raw_display_handle: None,
        }
    }
}

unsafe impl HasRawWindowHandle for WindowHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        let mut handle = WaylandWindowHandle::empty();
        handle.surface = self.wayland_window().wl_surface().id().as_ptr() as *mut _;
        RawWindowHandle::Wayland(handle)
    }
}

unsafe impl HasRawDisplayHandle for WindowHandle {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        let mut handle = WaylandDisplayHandle::empty();
        handle.display = self
            .raw_display_handle
            .expect("Window can only be created with a valid display pointer");
        RawDisplayHandle::Wayland(handle)
    }
}

#[derive(Clone)]
pub struct IdleHandle {
    window: WindowId,
    app: AppHandle,
}

impl IdleHandle {
    pub fn add_idle_callback<F>(&self, callback: F)
    where
        F: FnOnce(&mut dyn WinHandler) + Send + 'static,
    {
        self.add_idle_state_callback(|state| callback(&mut *state.handler))
    }

    fn add_idle_state_callback<F>(&self, callback: F)
    where
        F: FnOnce(&mut WindowState) + Send + 'static,
    {
        let window = self.window.clone();
        self.app.run_idle(move |state| {
            let win_state = state.windows.borrow_mut().get_mut(&window);
            if let Some(win_state) = win_state {
                callback(&mut *win_state);
            } else {
                tracing::error!("Ran add_idle_callback on a window which no longer exists")
            }
        });
    }

    pub fn add_idle_token(&self, token: IdleToken) {
        // TODO: Use a specialised type rather than dynamic dispatch for these tokens
        self.add_idle_callback(move |handler| handler.idle(token))
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
// TODO: According to https://github.com/linebender/druid/pull/2033, this should not be
// synced with the ID of the surface
pub(super) struct WindowId(ObjectId);

impl WindowId {
    pub fn new(surface: &impl WaylandSurface) -> Self {
        Self::of_surface(surface.wl_surface())
    }
    pub fn of_surface(surface: &WlSurface) -> Self {
        Self(surface.id().clone())
    }
}

/// The state associated with each window, stored in [`WaylandState`]
pub struct WindowState {
    handler: Box<dyn WinHandler>,
    wayland_window: Window,
    // TODO: Rc<RefCell>?
    properties: Arc<RwLock<WindowProperties>>,
}

#[derive(Clone)]
struct WindowProperties {
    // Requested size is used in configure, if it's supported
    requested_size: Option<Size>,
    // The dimensions of the surface we reported to the handler, and so report in get_size()
    // Wayland gives strong deference to the application on surface size
    // so, for example an application using wgpu could have the surface configured to be a different size
    current_size: Size,
    current_scale: Scale,
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
        // TODO: Support the fractional-scaling extension instead
        // This requires update in client-toolkit and wayland-protocols
        new_factor: i32,
    ) {
        let window = self.windows.get_mut(&WindowId::of_surface(surface));
        let window = window.expect("Should only get events for real windows");
        let factor = f64::from(new_factor);
        let scale = Scale::new(factor, factor);
        let new_size;
        {
            let mut props = window.properties.write().unwrap();
            // TODO: Effectively, we need to re-evaluate the size calculation
            // That means we need to cache the WindowConfigure or (mostly) equivalent
            let cur_size_raw = props.current_size.to_px(props.current_scale);
            new_size = cur_size_raw.to_dp(scale);
            props.current_scale = scale;
            props.current_size = new_size;
            // avoid locking the properties into user code
        }
        window.handler.scale(scale);
        window.handler.size(new_size)
    }

    fn frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &protocol::wl_surface::WlSurface,
        time: u32,
    ) {
        let Some(window) = self.windows.get_mut(&WindowId::of_surface(surface)) else { return };
        window.handler.prepare_paint();
        // TODO: Apply invalid properly
        let mut region = Region::EMPTY;
        // This is clearly very wrong, but might work for now :)
        region.add_rect(Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 5000.0,
            y1: 5000.0,
        });
        window.handler.paint(&region);
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
        let window: Option<&mut WindowState> = self.windows.get_mut(&WindowId::new(window));
    }
}
