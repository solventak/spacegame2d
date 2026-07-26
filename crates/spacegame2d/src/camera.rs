use glam::Vec2;

pub const VIEW_HEIGHT_METERS: f32 = 72.0;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    center: Vec2,
    arena_radius_meters: f32,
    drag_anchor_pixels: Option<Vec2>,
}

impl Camera {
    pub fn new(arena_radius_meters: f32) -> Self {
        Self {
            center: Vec2::ZERO,
            arena_radius_meters,
            drag_anchor_pixels: None,
        }
    }

    #[cfg(test)]
    pub fn center(self) -> Vec2 {
        self.center
    }

    pub fn viewport(self, width: u32, height: u32) -> [f32; 4] {
        let half = self.half_extents(width, height);
        [1.0 / half.x, 1.0 / half.y, self.center.x, self.center.y]
    }

    pub fn screen_to_world(self, cursor_pixels: Vec2, width: u32, height: u32) -> Vec2 {
        let half = self.half_extents(width, height);
        let width = width.max(1) as f32;
        let height = height.max(1) as f32;
        self.center
            + Vec2::new(
                (cursor_pixels.x / width * 2.0 - 1.0) * half.x,
                (1.0 - cursor_pixels.y / height * 2.0) * half.y,
            )
    }

    pub fn begin_drag(&mut self, cursor_pixels: Vec2) {
        self.drag_anchor_pixels = Some(cursor_pixels);
    }

    pub fn drag_to(&mut self, cursor_pixels: Vec2, width: u32, height: u32) -> bool {
        let Some(anchor) = self.drag_anchor_pixels else {
            return false;
        };
        self.drag_anchor_pixels = Some(cursor_pixels);
        let half = self.half_extents(width, height);
        let delta = cursor_pixels - anchor;
        self.center += Vec2::new(
            -delta.x * (half.x * 2.0 / width.max(1) as f32),
            delta.y * (half.y * 2.0 / height.max(1) as f32),
        );
        self.clamp(width, height);
        true
    }

    pub fn end_drag(&mut self) {
        self.drag_anchor_pixels = None;
    }

    pub fn clamp(&mut self, width: u32, height: u32) {
        let extent = self.half_extents(width, height) * 2.0;
        let limits = (Vec2::splat(self.arena_radius_meters) - extent / 6.0).max(Vec2::ZERO);
        self.center = self.center.clamp(-limits, limits);
    }

    fn half_extents(self, width: u32, height: u32) -> Vec2 {
        let half_height = VIEW_HEIGHT_METERS * 0.5;
        let aspect = width.max(1) as f32 / height.max(1) as f32;
        Vec2::new(half_height * aspect, half_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begins_centered_and_projects_with_aspect() {
        let camera = Camera::new(64.0);
        assert_eq!(camera.center(), Vec2::ZERO);
        assert_eq!(
            camera.screen_to_world(Vec2::new(400.0, 300.0), 800, 600),
            Vec2::ZERO
        );
        assert_eq!(
            camera.screen_to_world(Vec2::new(800.0, 0.0), 800, 600),
            Vec2::new(48.0, 36.0)
        );
    }

    #[test]
    fn drag_grabs_the_map_and_only_moves_while_active() {
        let mut camera = Camera::new(64.0);
        assert!(!camera.drag_to(Vec2::new(500.0, 350.0), 800, 600));
        assert!(!camera.drag_to(Vec2::new(600.0, 400.0), 800, 600));
        assert_eq!(camera.center(), Vec2::ZERO);
        camera.begin_drag(Vec2::new(400.0, 300.0));
        assert!(camera.drag_to(Vec2::new(500.0, 350.0), 800, 600));
        assert_eq!(camera.center(), Vec2::new(-12.0, 6.0));
        camera.end_drag();
        assert!(!camera.drag_to(Vec2::new(600.0, 400.0), 800, 600));
    }

    #[test]
    fn clamp_allows_one_third_of_each_visible_axis_outside() {
        let mut camera = Camera::new(64.0);
        camera.begin_drag(Vec2::ZERO);
        camera.drag_to(Vec2::new(-10_000.0, 10_000.0), 800, 600);
        let center = camera.center();
        assert!((center.x - 48.0).abs() < 0.001);
        assert!((center.y - 52.0).abs() < 0.001);
    }
}
