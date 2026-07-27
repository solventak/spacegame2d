use std::borrow::Cow;

use serde::Serialize;
use thiserror::Error;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::window::Window;
use wry::{
    Rect, WebView, WebViewBuilder,
    http::{Response, StatusCode},
};

use crate::{player_presentation::PlayerColor, session::UiStateEnvelope};

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

    fn initialization_script(state: &UiStateEnvelope) -> Result<String, serde_json::Error> {
        let state = serde_json::to_string(state)?;
        Ok(format!(
            "window.addEventListener('error', event => {{ document.title = `HUD_ERROR:${{event.message}}`; }});\
             window.addEventListener('unhandledrejection', event => {{ document.title = `HUD_ERROR:${{String(event.reason)}}`; }});\
             const listeners = new Set(); let current = {state};
             Object.defineProperty(window, '__SPACEGAME_HUD__', {{ value: Object.freeze({{
               getState: () => current,
               receive: value => {{ current = value; listeners.forEach(listener => listener(value)); }},
               subscribe: listener => {{ listeners.add(listener); return () => listeners.delete(listener); }},
               send: command => window.ipc.postMessage(JSON.stringify(command))
             }}), writable: false, configurable: false }});"
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudLayout {
    Pregame,
    Compact,
}

impl HudLayout {
    fn for_state(state: &UiStateEnvelope) -> Self {
        if state.state.is_connected() {
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
    #[error("failed to serialize HUD bootstrap model: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("failed to create HUD WebView: {0}")]
    Creation(#[from] wry::Error),
}

impl HudWebView {
    pub fn new<F>(window: &Window, state: &UiStateEnvelope, ipc: F) -> Result<Self, HudError>
    where
        F: Fn(String) + 'static,
    {
        let layout = HudLayout::for_state(state);
        let bounds = HudBounds::for_window(window.inner_size(), window.scale_factor(), layout);
        let script = LocalPlayerHudModel::initialization_script(state)?;
        let webview = WebViewBuilder::new()
            .with_url(format!("{HUD_ORIGIN}/index.html"))
            .with_transparent(true)
            .with_focused(false)
            .with_bounds(bounds.rect())
            .with_initialization_script(&script)
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

    pub fn publish(&mut self, window: &Window, state: &UiStateEnvelope) -> Result<(), HudError> {
        self.layout = HudLayout::for_state(state);
        self.resize(window)?;
        let state = serde_json::to_string(state)?;
        self.webview
            .evaluate_script(&format!("window.__SPACEGAME_HUD__.receive({state});"))?;
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
    fn model_is_camel_case_and_rust_publishes_the_colour() {
        assert_eq!(
            serde_json::to_string(&LocalPlayerHudModel::for_slot(2)).unwrap(),
            "{\"schemaVersion\":1,\"playerSlot\":2,\"color\":\"coral\",\"colorHex\":\"#FF6A47\"}"
        );
    }

    #[test]
    fn script_installs_an_immutable_namespace() {
        let state = crate::session::SessionLifecycle::<()>::new("localhost:4000").ui_state();
        let script = LocalPlayerHudModel::initialization_script(&state).unwrap();
        assert!(script.contains("Object.defineProperty(window, '__SPACEGAME_HUD__'"));
        assert!(script.contains("writable: false"));
        assert!(script.contains("\"kind\":\"disconnected\""));
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
