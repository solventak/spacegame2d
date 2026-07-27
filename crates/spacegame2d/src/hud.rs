use std::borrow::Cow;

#[cfg(target_os = "linux")]
use gdkx11::glib::Cast;
#[cfg(target_os = "linux")]
use gtk::prelude::{ContainerExt, GtkWindowExt, WidgetExt};
use serde::Serialize;
use thiserror::Error;
#[cfg(target_os = "linux")]
use winit::dpi::PhysicalPosition;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
#[cfg(target_os = "linux")]
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;
#[cfg(target_os = "linux")]
use wry::WebViewBuilderExtUnix;
use wry::{
    Rect, WebView, WebViewBuilder,
    http::{Response, StatusCode},
};

use crate::player_presentation::PlayerColor;

const HUD_ORIGIN: &str = "spacegame-hud://localhost";
const EDGE_INSET: f64 = 14.0;
const PREFERRED_WIDTH: f64 = 224.0;
// The Fleet panel uses 12px frame padding above and below its header/readout stack.
const PREFERRED_HEIGHT: f64 = 88.0;

const INDEX_HTML: &[u8] = include_bytes!("../hud/dist/index.html");
const HUD_JS: &[u8] = include_bytes!("../hud/dist/assets/hud.js");
const HUD_CSS: &[u8] = include_bytes!("../hud/dist/assets/hud.css");

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPlayerHudModel {
    pub schema_version: u8,
    pub player_slot: u32,
    pub color: PlayerColor,
    pub color_hex: &'static str,
}

impl LocalPlayerHudModel {
    pub fn for_slot(player_slot: u32) -> Self {
        let color = PlayerColor::for_slot(player_slot);
        Self {
            schema_version: 1,
            player_slot,
            color,
            color_hex: color.css_hex(),
        }
    }

    fn initialization_script(&self) -> Result<String, serde_json::Error> {
        let model = serde_json::to_string(self)?;
        #[cfg(target_os = "linux")]
        let host_style = "document.documentElement.dataset.spacegameHudHost = 'linux';";
        #[cfg(not(target_os = "linux"))]
        let host_style = "";
        Ok(format!(
            "{host_style}window.addEventListener('error', event => {{ document.title = `HUD_ERROR:${{event.message}}`; }});\
             window.addEventListener('unhandledrejection', event => {{ document.title = `HUD_ERROR:${{String(event.reason)}}`; }});\
             Object.defineProperty(window, '__SPACEGAME_HUD__', {{ value: Object.freeze({{ localPlayer: {model} }}), writable: false, configurable: false }});"
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl HudBounds {
    pub fn for_window(size: PhysicalSize<u32>, scale_factor: f64) -> Self {
        let scale = scale_factor.max(f64::MIN_POSITIVE);
        let parent_width = f64::from(size.width) / scale;
        let parent_height = f64::from(size.height) / scale;
        let width = PREFERRED_WIDTH.min(parent_width.max(1.0));
        let height = PREFERRED_HEIGHT.min(parent_height.max(1.0));
        Self {
            x: EDGE_INSET.min((parent_width - width).max(0.0)),
            y: EDGE_INSET.min((parent_height - height).max(0.0)),
            width,
            height,
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn rect(self) -> Rect {
        Rect {
            position: LogicalPosition::new(self.x, self.y).into(),
            size: LogicalSize::new(self.width, self.height).into(),
        }
    }

    #[cfg(target_os = "linux")]
    fn host_rect(self) -> Rect {
        Rect {
            position: LogicalPosition::new(0.0, 0.0).into(),
            size: LogicalSize::new(self.width, self.height).into(),
        }
    }
}

pub struct HudWebView {
    // Keep the WebView before its GTK host so WebKit drops before the host window.
    _webview: WebView,
    #[cfg(target_os = "linux")]
    host_window: gtk::Window,
    #[cfg(target_os = "linux")]
    _parent_window: gdkx11::X11Window,
    bounds: HudBounds,
}

#[derive(Debug, Error)]
pub enum HudError {
    #[error("failed to serialize HUD bootstrap model: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("failed to create HUD WebView: {0}")]
    Creation(#[from] wry::Error),
    #[cfg(target_os = "linux")]
    #[error("failed to access the game window's X11 handle: {0}")]
    WindowHandle(#[from] winit::raw_window_handle::HandleError),
    #[cfg(target_os = "linux")]
    #[error("failed to create Linux HUD host: {0}")]
    LinuxHost(&'static str),
}

impl HudWebView {
    #[cfg(target_os = "linux")]
    pub fn new(window: &Window, model: LocalPlayerHudModel) -> Result<Self, HudError> {
        let bounds = HudBounds::for_window(window.inner_size(), window.scale_factor());
        let parent_window = foreign_x11_window(window)?;
        let host_window = gtk::Window::new(gtk::WindowType::Toplevel);
        configure_host_window(&host_window);
        let container = gtk::Fixed::new();
        host_window.add(&container);
        host_window.realize();
        let host_gdk_window = host_window
            .window()
            .ok_or(HudError::LinuxHost("GTK did not realize the HUD window"))?;
        host_gdk_window.set_transient_for(parent_window.upcast_ref());
        host_gdk_window.input_shape_combine_region(&gtk::cairo::Region::create(), 0, 0);
        let script = model.initialization_script()?;
        let webview = WebViewBuilder::new()
            .with_url(format!("{HUD_ORIGIN}/index.html"))
            // This is a complete, compact panel in its own window. Keeping it opaque avoids
            // per-frame composition work against the WGPU game surface.
            .with_transparent(false)
            .with_focused(false)
            .with_bounds(bounds.host_rect())
            .with_initialization_script(&script)
            .with_on_page_load_handler(|event, url| {
                let phase = match event {
                    wry::PageLoadEvent::Started => "started",
                    wry::PageLoadEvent::Finished => "finished",
                };
                tracing::info!(event = "hud_page_load", phase, %url);
            })
            .with_document_title_changed_handler(|title| {
                tracing::info!(event = "hud_document_status", %title);
            })
            .with_navigation_handler(|url| allowed_navigation(url.as_str()))
            .with_custom_protocol("spacegame-hud".into(), move |_id, request| {
                let (status, mime, body) = embedded_asset(request.uri().path());
                tracing::info!(
                    event = "hud_asset_request",
                    path = request.uri().path(),
                    status = status.as_u16(),
                );
                Response::builder()
                    .status(status)
                    .header("Content-Type", mime)
                    .body(Cow::Borrowed(body))
                    .expect("embedded HUD response is valid")
            })
            .build_gtk(&container)?;
        let mut hud = Self {
            _webview: webview,
            host_window,
            _parent_window: parent_window,
            bounds,
        };
        hud.resize(window)?;
        hud.host_window.show_all();
        Ok(hud)
    }

    #[cfg(not(target_os = "linux"))]
    pub fn new(window: &Window, model: LocalPlayerHudModel) -> Result<Self, HudError> {
        let bounds = HudBounds::for_window(window.inner_size(), window.scale_factor());
        let script = model.initialization_script()?;
        let webview = WebViewBuilder::new()
            .with_url(format!("{HUD_ORIGIN}/index.html"))
            .with_transparent(true)
            .with_focused(false)
            .with_bounds(bounds.rect())
            .with_initialization_script(&script)
            .with_on_page_load_handler(|event, url| {
                let phase = match event {
                    wry::PageLoadEvent::Started => "started",
                    wry::PageLoadEvent::Finished => "finished",
                };
                tracing::info!(event = "hud_page_load", phase, %url);
            })
            .with_document_title_changed_handler(|title| {
                tracing::info!(event = "hud_document_status", %title);
            })
            .with_navigation_handler(|url| allowed_navigation(url.as_str()))
            .with_custom_protocol("spacegame-hud".into(), move |_id, request| {
                let (status, mime, body) = embedded_asset(request.uri().path());
                tracing::info!(
                    event = "hud_asset_request",
                    path = request.uri().path(),
                    status = status.as_u16(),
                );
                Response::builder()
                    .status(status)
                    .header("Content-Type", mime)
                    .body(Cow::Borrowed(body))
                    .expect("embedded HUD response is valid")
            })
            .build_as_child(window)?;
        Ok(Self {
            _webview: webview,
            bounds,
        })
    }

    pub fn resize(&mut self, window: &Window) -> Result<(), HudError> {
        let bounds = HudBounds::for_window(window.inner_size(), window.scale_factor());
        #[cfg(target_os = "linux")]
        {
            let position = host_position(window, bounds, self.host_window.scale_factor())?;
            self.host_window.move_(position.x, position.y);
            self.host_window
                .resize(bounds.width.round() as i32, bounds.height.round() as i32);
        }
        #[cfg(not(target_os = "linux"))]
        if bounds != self.bounds {
            self._webview.set_bounds(bounds.rect())?;
        }
        self.bounds = bounds;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn set_visible(&self, visible: bool) {
        self.host_window.set_visible(visible);
    }
}

#[cfg(target_os = "linux")]
fn foreign_x11_window(window: &Window) -> Result<gdkx11::X11Window, HudError> {
    let RawWindowHandle::Xlib(handle) = window.window_handle()?.as_raw() else {
        return Err(HudError::LinuxHost(
            "the Winit game window is not an X11 window",
        ));
    };
    let display = gtk::gdk::Display::default()
        .ok_or(HudError::LinuxHost("GDK did not provide an X11 display"))?
        .downcast::<gdkx11::X11Display>()
        .map_err(|_| HudError::LinuxHost("GDK is not using the X11 backend"))?;
    Ok(gdkx11::X11Window::foreign_new_for_display(
        &display,
        handle.window as _,
    ))
}

#[cfg(target_os = "linux")]
fn configure_host_window(window: &gtk::Window) {
    use gtk::prelude::*;

    window.set_decorated(false);
    window.set_resizable(false);
    window.set_accept_focus(false);
    window.set_focus_on_map(false);
    window.set_skip_taskbar_hint(true);
    window.set_skip_pager_hint(true);
    window.set_type_hint(gtk::gdk::WindowTypeHint::Utility);
}

#[cfg(target_os = "linux")]
fn host_position(
    window: &Window,
    bounds: HudBounds,
    host_scale_factor: i32,
) -> Result<LogicalPosition<i32>, HudError> {
    let parent_position = window
        .inner_position()
        .map_err(|_| HudError::LinuxHost("Winit could not determine the game client position"))?;
    let scale = window.scale_factor().max(f64::MIN_POSITIVE);
    let physical = PhysicalPosition::new(
        parent_position.x + (bounds.x * scale).round() as i32,
        parent_position.y + (bounds.y * scale).round() as i32,
    );
    Ok(physical.to_logical(f64::from(host_scale_factor.max(1))))
}

fn embedded_asset(path: &str) -> (StatusCode, &'static str, &'static [u8]) {
    match path {
        "/" | "/index.html" => (StatusCode::OK, "text/html; charset=utf-8", INDEX_HTML),
        "/assets/hud.js" => (StatusCode::OK, "text/javascript; charset=utf-8", HUD_JS),
        "/assets/hud.css" => (StatusCode::OK, "text/css; charset=utf-8", HUD_CSS),
        _ => (
            StatusCode::NOT_FOUND,
            "text/plain; charset=utf-8",
            b"Not found",
        ),
    }
}

fn allowed_navigation(url: &str) -> bool {
    url == HUD_ORIGIN
        || url.starts_with(&format!("{HUD_ORIGIN}/"))
        || url == "http://spacegame-hud.localhost"
        || url.starts_with("http://spacegame-hud.localhost/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_is_camel_case_and_rust_publishes_the_colour() {
        assert_eq!(
            serde_json::to_string(&LocalPlayerHudModel::for_slot(2)).unwrap(),
            "{\"schemaVersion\":1,\"playerSlot\":2,\"color\":\"coral\",\"colorHex\":\"#FF6A47\"}"
        );
    }

    #[test]
    fn script_installs_an_immutable_namespace() {
        let script = LocalPlayerHudModel::for_slot(1)
            .initialization_script()
            .unwrap();
        assert!(script.contains("Object.defineProperty(window, '__SPACEGAME_HUD__'"));
        assert!(script.contains("writable: false"));
        assert!(script.contains("\"color\":\"cyan\""));
    }

    #[test]
    fn assets_and_navigation_are_strictly_local() {
        assert_eq!(
            embedded_asset("/assets/hud.css").1,
            "text/css; charset=utf-8"
        );
        assert_eq!(embedded_asset("/nope").0, StatusCode::NOT_FOUND);
        assert!(allowed_navigation("spacegame-hud://localhost/index.html"));
        assert!(allowed_navigation(
            "http://spacegame-hud.localhost/assets/hud.js"
        ));
        assert!(!allowed_navigation("https://example.com"));
    }

    #[test]
    fn bounds_are_dpi_aware_and_clamped() {
        assert_eq!(
            HudBounds::for_window(PhysicalSize::new(800, 600), 1.0),
            HudBounds {
                x: 14.0,
                y: 14.0,
                width: 224.0,
                height: 88.0
            }
        );
        assert_eq!(
            HudBounds::for_window(PhysicalSize::new(100, 50), 1.0),
            HudBounds {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0
            }
        );
        assert_eq!(
            HudBounds::for_window(PhysicalSize::new(500, 250), 2.0).x,
            14.0
        );
    }
}
