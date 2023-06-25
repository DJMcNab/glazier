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

use std::cell::RefCell;
use std::marker::PhantomData;
use std::os::raw::c_void;
use std::rc::{Rc, Weak};
use std::sync::mpsc::{self, Sender};

use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
    WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::compositor::CompositorHandler;
use smithay_client_toolkit::reexports::calloop::channel;
use smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface;
use smithay_client_toolkit::reexports::client::{protocol, Connection, Proxy, QueueHandle};
use smithay_client_toolkit::shell::xdg::window::{DecorationMode, Window, WindowHandler};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::{delegate_compositor, delegate_xdg_shell, delegate_xdg_window};
use tracing;
use wayland_backend::client::ObjectId;

use super::application::{self};
use super::menu::Menu;
use super::{ActiveAction, IdleAction, WaylandState};

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
use crate::{IdleToken, Region};

#[derive(Clone)]
pub struct WindowHandle {
    idle_sender: Sender<IdleAction>,
    loop_sender: channel::Sender<ActiveAction>,
    properties: Weak<RefCell<WindowProperties>>,
    // Safety: Points to a wl_display instance
    raw_display_handle: Option<*mut c_void>,
    not_send: PhantomData<*mut ()>,
}

impl WindowHandle {
    fn id(&self) -> WindowId {
        let props = self.properties();
        WindowId::new(props.wayland_window())
    }

    fn defer(&self, action: WindowAction) {
        self.loop_sender
            .send(ActiveAction::Window(self.id(), action))
            .expect("Running on a window should only occur whilst application is active")
    }

    fn properties(&self) -> std::cell::Ref<WindowProperties> {
        self.properties.upgrade().unwrap().borrow()
    }

    fn properties_mut(&self) -> std::cell::RefMut<WindowProperties> {
        self.properties.upgrade().unwrap().borrow_mut()
    }

    pub fn show(&self) {
        tracing::debug!("show initiated");
        let props = self.properties();
        props.wayland_window().commit();
    }

    pub fn resizable(&self, resizable: bool) {
        tracing::warn!("resizable is unimplemented on wayland");
        // TODO: If we are using fallback decorations, we should be able to disable
        // dragging based resizing
    }

    pub fn show_titlebar(&self, show_titlebar: bool) {
        tracing::info!("show_titlebar is implemented on a best-effort basis on wayland");
        // TODO: Track this into the fallback decorations when we add those
        let props = self.properties();
        if show_titlebar {
            props
                .wayland_window()
                .request_decoration_mode(Some(DecorationMode::Server))
        } else {
            props
                .wayland_window()
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
        let mut props = self.properties_mut();
        props.requested_size = Some(size);

        // We don't need to tell the server about changing the size - so long as the size of the surface gets changed properly
        // So, all we need to do is to tell the handler about this change (after caching it here)
        // We must defer this, because we're probably in the handler
        self.defer(WindowAction::ResizeRequested);
    }

    pub fn get_size(&self) -> Size {
        let props = self.properties();
        props.current_size
    }

    pub fn set_window_state(&mut self, state: window::WindowState) {
        let props = self.properties();
        match state {
            crate::WindowState::Maximized => props.wayland_window().set_maximized(),
            crate::WindowState::Minimized => props.wayland_window().set_minimized(),
            // TODO: I don't think we can do much better than this - we can't unset being minimised
            crate::WindowState::Restored => props.wayland_window().unset_maximized(),
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
        self.defer(WindowAction::Close)
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
            idle_sender: self.idle_sender.clone(),
            window: self.id(),
        })
    }

    /// Get the `Scale` of the window.
    pub fn get_scale(&self) -> Result<Scale, ShellError> {
        let props = self.properties();
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
        // Make fake channels, to work around WindowHandle being default
        let (idle_sender, _) = mpsc::channel();
        let (loop_sender, _) = channel::channel();
        // TODO: Why is this Default?
        WindowHandle {
            not_send: Default::default(),
            wayland_window: None,
            properties: Weak::new(),
            raw_display_handle: None,
            idle_sender,
            loop_sender,
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
    idle_sender: Sender<IdleAction>,
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
        self.idle_sender
            .send(IdleAction::Callback(Box::new(move |state| {
                let win_state = state.windows.get_mut(&window);
                if let Some(win_state) = win_state {
                    callback(&mut *win_state);
                } else {
                    tracing::error!("Ran add_idle_callback on a window which no longer exists")
                }
            })));
    }

    pub fn add_idle_token(&self, token: IdleToken) {
        match self
            .idle_sender
            .send(IdleAction::Token(self.window.clone(), token))
        {
            Ok(()) => (),
            Err(err) => tracing::warn!("Requested idle on invalid application: {err:?}"),
        }
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
pub(super) struct WindowState {
    pub handler: Box<dyn WinHandler>,
    wayland_window: Window,
    // TODO: Rc<RefCell>?
    properties: Rc<RefCell<WindowProperties>>,
}

struct WindowProperties {
    // Requested size is used in configure, if it's supported
    requested_size: Option<Size>,
    // The dimensions of the surface we reported to the handler, and so report in get_size()
    // Wayland gives strong deference to the application on surface size
    // so, for example an application using wgpu could have the surface configured to be a different size
    current_size: Size,
    current_scale: Scale,
    // The underlying wayland Window
    // The way to close this Window is to drop the handle
    // We make this the only handle, so we can definitely drop it
    wayland_window: Option<Window>,
}

impl WindowProperties {
    fn wayland_window(&self) -> &Window {
        self.wayland_window
            .as_ref()
            .expect("Shouldn't operate on closed window")
    }
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
        // This requires an update in client-toolkit and wayland-protocols
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
        _: &Connection,
        _: &QueueHandle<Self>,
        wl_window: &smithay_client_toolkit::shell::xdg::window::Window,
    ) {
        let Some(window)= self.windows.get_mut(&WindowId::new(wl_window)) else { return };
        window.handler.request_close();
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

pub(super) enum WindowAction {
    /// Change the window size, based on `requested_size`
    ///
    /// `requested_size` must be set before this is called
    ResizeRequested,
    /// Close the Window
    Close,
}

impl WindowAction {
    pub(super) fn run(self, state: &mut WaylandState, window_id: WindowId) {
        match self {
            WindowAction::ResizeRequested => {
                let window = {
                    let Some(window) = state.windows.get_mut(&window_id) else { return };

                    let mut props = window.properties.borrow_mut();
                    // TODO: Should this requested_size be taken?
                    // Reason to suspect it should be would be resizes (if enabled)
                    let size = props.requested_size.expect("Can't unset requested size");
                    props.current_size = size;
                    // TODO: Ensure we follow the rules laid out by the compositor in `configure`
                    window.handler.size(size);
                    window.wayland_window.clone()
                };
                // TODO: Don't stack up frame callbacks - need to ensure only one per `paint` call?
                let surface = window.wl_surface();
                // Request a redraw now that the size has changed
                surface.frame(&state.wayland_queue.clone(), surface.clone());
            }
            WindowAction::Close => {
                // Remove the window from tracking
                let Some(window) = state.windows.remove(&window_id) else {
                    tracing::error!("Tried to close the same window twice");
                    return;
                };
                // We will drop the proper wayland window later
                let mut props = window.properties.borrow_mut();
                if state.windows.is_empty() {
                    state.loop_signal.stop();
                }
            }
        }
    }
}
