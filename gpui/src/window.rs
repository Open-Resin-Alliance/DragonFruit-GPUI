//! The window renderer: `WgpuRenderer`, so `gpui_interop` can hand the viewport
//! the window's own device.

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use gpui::{DevicePixels, WindowOptions, size};
use gpui_wgpu::{WgpuContextSlot, WgpuRenderer, WgpuSurfaceConfig};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};

pub fn with_wgpu_renderer(options: WindowOptions) -> WindowOptions {
    options.with_renderer_factory(gpui::FnRendererFactory(
        |target: gpui::RendererTarget<'_>| -> Result<Box<dyn gpui::PlatformRenderer>> {
            let handles = Handles {
                window: target.window_handle,
                display: target.display_handle,
            };
            let config = WgpuSurfaceConfig {
                size: size(
                    DevicePixels((target.size.width.0 * target.scale_factor).round() as i32),
                    DevicePixels((target.size.height.0 * target.scale_factor).round() as i32),
                ),
                transparent: target.transparent,
                preferred_present_mode: None,
            };
            let slot: WgpuContextSlot = Rc::new(RefCell::new(None));
            let renderer =
                WgpuRenderer::new(slot, &handles, config, None, gpui::GpuPreference::Auto)?;
            Ok(Box::new(renderer))
        },
    ))
}

#[derive(Clone, Copy, Debug)]
struct Handles {
    window: Option<RawWindowHandle>,
    display: Option<RawDisplayHandle>,
}

// Safety: the handles name process-global native objects that outlive the
// renderer, the same guarantee `gpui_wgpu`'s own `RawWindowHandles` makes.
unsafe impl Send for Handles {}
unsafe impl Sync for Handles {}

impl HasWindowHandle for Handles {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        match self.window {
            Some(raw) => Ok(unsafe { WindowHandle::borrow_raw(raw) }),
            None => Err(HandleError::Unavailable),
        }
    }
}

impl HasDisplayHandle for Handles {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        match self.display {
            Some(raw) => Ok(unsafe { DisplayHandle::borrow_raw(raw) }),
            None => Err(HandleError::Unavailable),
        }
    }
}
