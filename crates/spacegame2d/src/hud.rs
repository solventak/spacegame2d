use std::borrow::Cow;

use spacegame2d_ui_protocol::{ConnectionStateSnapshot, EngineToUiMessage};
use thiserror::Error;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::window::Window;
use wry::{
    Rect, WebView, WebViewBuilder,
    http::{Response, StatusCode},
};

const HUD_ORIGIN: &str = "spacegame-hud://localhost";
const EDGE_INSET: f64 = 14.0;
const PREFERRED_WIDTH: f64 = 224.0;
// The Fleet panel uses 12px frame padding above and below its header/readout stack.
const PREFERRED_HEIGHT: f64 = 88.0;

const INDEX_HTML: &[u8] = include_bytes!("../hud/dist/index.html");
const HUD_JS: &[u8] = include_bytes!("../hud/dist/assets/hud.js");
const HUD_CSS: &[u8] = include_bytes!("../hud/dist/assets/hud.css");

fn initialization_script() -> &'static str {
    "window.addEventListener('error', event => { document.title = `HUD_ERROR:${event.message}`; });\
     window.addEventListener('unhandledrejection', event => { document.title = `HUD_ERROR:${String(event.reason)}`; });\
     const listeners = new Set();\
     Object.defineProperty(window, '__SPACEGAME_HUD__', { value: Object.freeze({\
       receiveJson: value => { listeners.forEach(listener => listener(value)); },\
       subscribe: listener => { listeners.add(listener); return () => listeners.delete(listener); },\
       sendJson: value => window.ipc.postMessage(value)\
     }), writable: false, configurable: false });"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudLayout {
    Pregame,
    Compact,
}

impl HudLayout {
    fn for_state(state: &ConnectionStateSnapshot) -> Self {
        if state.is_connected() {
            Self::Compact
        } else {
            Self::Pregame
        }
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
    pub fn for_window(size: PhysicalSize<u32>, scale_factor: f64, layout: HudLayout) -> Self {
        let scale = scale_factor.max(f64::MIN_POSITIVE);
        let parent_width = f64::from(size.width) / scale;
        let parent_height = f64::from(size.height) / scale;
        if layout == HudLayout::Pregame {
            return Self {
                x: 0.0,
                y: 0.0,
                width: parent_width.max(1.0),
                height: parent_height.max(1.0),
            };
        }
        let width = PREFERRED_WIDTH.min(parent_width.max(1.0));
        let height = PREFERRED_HEIGHT.min(parent_height.max(1.0));
        Self {
            x: EDGE_INSET.min((parent_width - width).max(0.0)),
            y: EDGE_INSET.min((parent_height - height).max(0.0)),
            width,
            height,
        }
    }

    fn rect(self) -> Rect {
        Rect {
            position: LogicalPosition::new(self.x, self.y).into(),
            size: LogicalSize::new(self.width, self.height).into(),
        }
    }
}

pub struct HudWebView {
    // Keep the child WebView ahead of its Winit parent-window owner so it drops first.
    webview: WebView,
    bounds: HudBounds,
    layout: HudLayout,
}

#[derive(Debug, Error)]
pub enum HudError {
    #[error("failed to serialize HUD message: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("failed to create HUD WebView: {0}")]
    Creation(#[from] wry::Error),
}

impl HudWebView {
    pub fn new<F>(window: &Window, ipc: F) -> Result<Self, HudError>
    where
        F: Fn(String) + 'static,
    {
        let layout = HudLayout::Pregame;
        let bounds = HudBounds::for_window(window.inner_size(), window.scale_factor(), layout);
        let webview = WebViewBuilder::new()
            .with_url(format!("{HUD_ORIGIN}/index.html"))
            .with_transparent(true)
            .with_focused(false)
            .with_bounds(bounds.rect())
            .with_initialization_script(initialization_script())
            .with_ipc_handler(move |request| ipc(request.body().clone()))
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
            webview,
            bounds,
            layout,
        })
    }

    pub fn resize(&mut self, window: &Window) -> Result<(), HudError> {
        let bounds = HudBounds::for_window(window.inner_size(), window.scale_factor(), self.layout);
        if bounds != self.bounds {
            self.webview.set_bounds(bounds.rect())?;
        }
        self.bounds = bounds;
        Ok(())
    }

    pub fn publish(
        &mut self,
        window: &Window,
        message: &EngineToUiMessage,
    ) -> Result<(), HudError> {
        if let EngineToUiMessage::ConnectionStateChanged { state, .. } = message {
            self.layout = HudLayout::for_state(state);
        }
        self.resize(window)?;
        let state = message.encode()?;
        let argument = serde_json::to_string(&state)?;
        self.webview.evaluate_script(&format!(
            "window.__SPACEGAME_HUD__.receiveJson({argument});"
        ))?;
        Ok(())
    }
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
    fn script_installs_an_immutable_namespace() {
        let script = initialization_script();
        assert!(script.contains("Object.defineProperty(window, '__SPACEGAME_HUD__'"));
        assert!(script.contains("writable: false"));
        assert!(script.contains("receiveJson"));
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
            HudBounds::for_window(PhysicalSize::new(800, 600), 2.0, HudLayout::Pregame),
            HudBounds {
                x: 0.0,
                y: 0.0,
                width: 400.0,
                height: 300.0,
            }
        );
        assert_eq!(
            HudBounds::for_window(PhysicalSize::new(800, 600), 1.0, HudLayout::Compact),
            HudBounds {
                x: 14.0,
                y: 14.0,
                width: 224.0,
                height: 88.0
            }
        );
        assert_eq!(
            HudBounds::for_window(PhysicalSize::new(100, 50), 1.0, HudLayout::Compact),
            HudBounds {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0
            }
        );
        assert_eq!(
            HudBounds::for_window(PhysicalSize::new(500, 250), 2.0, HudLayout::Compact).x,
            14.0
        );
    }
}
