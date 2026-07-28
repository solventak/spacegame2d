use std::borrow::Cow;

use spacegame2d_ui_protocol::{EngineToUiMessage, MatchSessionState};
use thiserror::Error;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::window::Window;
use wry::{
    Rect, WebView, WebViewBuilder,
    http::{Response, StatusCode},
};

const HUD_ORIGIN: &str = "spacegame-hud://localhost";
const REVEAL_BAND_HEIGHT: f64 = 112.0;
const COMPACT_BAR_HEIGHT: f64 = 34.0;
const JOIN_HOLD: std::time::Duration = std::time::Duration::from_millis(700);
const DOCK_DURATION: std::time::Duration = std::time::Duration::from_millis(600);

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
    Join,
    Docking,
    Compact,
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
        if matches!(layout, HudLayout::Join | HudLayout::Docking) {
            let height = REVEAL_BAND_HEIGHT.min(parent_height.max(1.0));
            return Self {
                x: 0.0,
                y: if layout == HudLayout::Join {
                    ((parent_height - height) / 2.0).max(0.0)
                } else {
                    0.0
                },
                width: parent_width.max(1.0),
                height,
            };
        }
        let height = COMPACT_BAR_HEIGHT.min(parent_height.max(1.0));
        Self {
            x: 0.0,
            y: 0.0,
            width: parent_width.max(1.0),
            height,
        }
    }

    fn rect(self) -> Rect {
        Rect {
            position: LogicalPosition::new(self.x, self.y).into(),
            size: LogicalSize::new(self.width, self.height).into(),
        }
    }

    fn move_vertical(from: Self, to_y: f64, progress: f64) -> Self {
        let t = progress.clamp(0.0, 1.0);
        Self {
            y: from.y + (to_y - from.y) * t,
            ..from
        }
    }
}

pub struct HudWebView {
    // Keep the child WebView ahead of its Winit parent-window owner so it drops first.
    webview: WebView,
    bounds: HudBounds,
    layout: HudLayout,
    transition_started: Option<std::time::Instant>,
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
            transition_started: None,
        })
    }

    pub fn resize(&mut self, window: &Window) -> Result<(), HudError> {
        let bounds = self.current_bounds(window, std::time::Instant::now());
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
        let state = message.encode()?;
        let argument = serde_json::to_string(&state)?;
        let match_class_script = match message {
            EngineToUiMessage::MatchSessionStateChanged {
                state: MatchSessionState::Active { .. },
                ..
            } => "document.documentElement.classList.add('match-active');",
            EngineToUiMessage::MatchSessionStateChanged { .. } => {
                "document.documentElement.classList.remove('match-active');"
            }
            _ => "",
        };
        self.webview.evaluate_script(&format!(
            "{match_class_script}window.__SPACEGAME_HUD__.receiveJson({argument});"
        ))?;
        if let EngineToUiMessage::MatchSessionStateChanged { state, .. } = message {
            self.apply_match_state(state);
        }
        self.resize(window)?;
        Ok(())
    }

    pub fn advance(&mut self, window: &Window, now: std::time::Instant) -> Result<(), HudError> {
        let Some(started) = self.transition_started else {
            return Ok(());
        };
        self.layout = layout_at(started, now);
        if self.layout == HudLayout::Compact {
            self.transition_started = None;
        }
        let bounds = self.current_bounds(window, now);
        if bounds != self.bounds {
            self.webview.set_bounds(bounds.rect())?;
            self.bounds = bounds;
        }
        Ok(())
    }

    pub fn next_deadline(&self) -> Option<std::time::Instant> {
        self.transition_started.map(|started| {
            if self.layout == HudLayout::Join {
                started + JOIN_HOLD
            } else {
                std::time::Instant::now() + std::time::Duration::from_millis(16)
            }
        })
    }

    fn apply_match_state(&mut self, state: &MatchSessionState) {
        match state {
            MatchSessionState::Active { .. } if self.layout == HudLayout::Pregame => {
                self.layout = HudLayout::Join;
                self.transition_started = Some(std::time::Instant::now());
            }
            MatchSessionState::Reset { .. } | MatchSessionState::Waiting { .. } => {
                self.layout = HudLayout::Pregame;
                self.transition_started = None;
            }
            MatchSessionState::Active { .. } => {}
        }
    }

    fn current_bounds(&self, window: &Window, now: std::time::Instant) -> HudBounds {
        let size = window.inner_size();
        let scale = window.scale_factor();
        if self.layout != HudLayout::Docking {
            return HudBounds::for_window(size, scale, self.layout);
        }
        let started = self.transition_started.unwrap_or(now);
        let elapsed = now.saturating_duration_since(started);
        let progress =
            elapsed.saturating_sub(JOIN_HOLD).as_secs_f64() / DOCK_DURATION.as_secs_f64();
        let eased = ease_out(progress);
        HudBounds::move_vertical(
            HudBounds::for_window(size, scale, HudLayout::Join),
            0.0,
            eased,
        )
    }
}

fn ease_out(value: f64) -> f64 {
    let t = value.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

fn layout_at(started: std::time::Instant, now: std::time::Instant) -> HudLayout {
    let elapsed = now.saturating_duration_since(started);
    if elapsed < JOIN_HOLD {
        HudLayout::Join
    } else if elapsed < JOIN_HOLD + DOCK_DURATION {
        HudLayout::Docking
    } else {
        HudLayout::Compact
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
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 34.0
            }
        );
        assert_eq!(
            HudBounds::for_window(PhysicalSize::new(100, 50), 1.0, HudLayout::Compact),
            HudBounds {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 34.0
            }
        );
        assert_eq!(
            HudBounds::for_window(PhysicalSize::new(500, 250), 2.0, HudLayout::Compact).width,
            250.0
        );
        assert_eq!(
            HudBounds::for_window(PhysicalSize::new(800, 600), 1.0, HudLayout::Join),
            HudBounds {
                x: 0.0,
                y: 244.0,
                width: 800.0,
                height: 112.0,
            }
        );
    }

    #[test]
    fn docking_moves_the_fixed_reveal_band_without_resizing() {
        let size = PhysicalSize::new(1600, 900);
        let join = HudBounds::for_window(size, 1.0, HudLayout::Join);
        let halfway = HudBounds::move_vertical(join, 0.0, ease_out(0.5));
        assert_eq!(halfway.x, 0.0);
        assert_eq!(halfway.width, 1600.0);
        assert_eq!(halfway.height, REVEAL_BAND_HEIGHT);
        assert!(halfway.y > 0.0);
        assert!(halfway.y < join.y);

        let docked = HudBounds::move_vertical(join, 0.0, 1.0);
        assert_eq!(docked.y, 0.0);
        assert_eq!(docked.width, join.width);
        assert_eq!(docked.height, join.height);
    }

    #[test]
    fn join_transition_holds_then_docks_and_settles() {
        let started = std::time::Instant::now();
        assert_eq!(layout_at(started, started), HudLayout::Join);
        assert_eq!(
            layout_at(
                started,
                started + JOIN_HOLD - std::time::Duration::from_millis(1)
            ),
            HudLayout::Join
        );
        assert_eq!(layout_at(started, started + JOIN_HOLD), HudLayout::Docking);
        assert_eq!(
            layout_at(
                started,
                started + JOIN_HOLD + DOCK_DURATION - std::time::Duration::from_millis(1)
            ),
            HudLayout::Docking
        );
        assert_eq!(
            layout_at(started, started + JOIN_HOLD + DOCK_DURATION),
            HudLayout::Compact
        );
    }
}
