use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use math;
use math::MetricSpace;

pub use application::event::MouseButton;

/// The setup parameters of mouse device.
///
/// Notes that the `distance` series paramters are measured in points.
#[derive(Debug, Clone, Copy)]
pub struct MouseParams {
    pub press_timeout: Duration,
    pub max_press_distance: f32,

    pub click_timeout: Duration,
    pub max_click_distance: f32,
}

impl Default for MouseParams {
    fn default() -> Self {
        MouseParams {
            press_timeout: Duration::from_millis(500),
            max_press_distance: 25.0,

            click_timeout: Duration::from_millis(500),
            max_click_distance: 25.0,
        }
    }
}

pub struct Mouse {
    downs: HashSet<MouseButton>,
    presses: HashSet<MouseButton>,
    releases: HashSet<MouseButton>,
    last_position: math::Vector2<f32>,
    position: math::Vector2<f32>,
    scrol: math::Vector2<f32>,
    click_detectors: HashMap<MouseButton, ClickDetector>,
    params: MouseParams,
}

impl Mouse {
    pub fn new(params: MouseParams) -> Self {
        Mouse {
            downs: HashSet::new(),
            presses: HashSet::new(),
            releases: HashSet::new(),
            last_position: math::Vector2::new(0.0, 0.0),
            position: math::Vector2::new(0.0, 0.0),
            scrol: math::Vector2::new(0.0, 0.0),
            click_detectors: HashMap::new(),
            params: params,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.downs.clear();
        self.presses.clear();
        self.releases.clear();
        self.last_position = math::Vector2::new(0.0, 0.0);
        self.position = math::Vector2::new(0.0, 0.0);
        self.scrol = math::Vector2::new(0.0, 0.0);

        for v in self.click_detectors.values_mut() {
            v.reset();
        }
    }

    #[inline]
    pub fn advance(&mut self) {
        self.presses.clear();
        self.releases.clear();
        self.scrol = math::Vector2::new(0.0, 0.0);
        self.last_position = self.position;

        for v in self.click_detectors.values_mut() {
            v.advance();
        }
    }

    #[inline]
    pub fn on_move(&mut self, position: (f32, f32)) {
        self.position = position.into();
    }

    #[inline]
    pub fn on_button_pressed(&mut self, button: MouseButton) {
        if !self.downs.contains(&button) {
            self.downs.insert(button);
            self.presses.insert(button);
        }

        if let Some(detector) = self.click_detectors.get_mut(&button) {
            detector.on_pressed(self.position);
            return;
        }

        let mut detector = ClickDetector::new(self.params);
        detector.on_pressed(self.position);
        self.click_detectors.insert(button, detector);
    }

    #[inline]
    pub fn on_button_released(&mut self, button: MouseButton) {
        self.downs.remove(&button);
        self.releases.insert(button);

        if let Some(detector) = self.click_detectors.get_mut(&button) {
            detector.on_released(self.position);
            return;
        }

        let mut detector = ClickDetector::new(self.params);
        detector.on_released(self.position);
        self.click_detectors.insert(button, detector);
    }

    #[inline]
    pub fn on_wheel_scroll(&mut self, delta: (f32, f32)) {
        self.scrol = delta.into();
    }

    #[inline]
    pub fn is_button_down(&self, button: MouseButton) -> bool {
        self.downs.contains(&button)
    }

    #[inline]
    pub fn is_button_press(&self, button: MouseButton) -> bool {
        self.presses.contains(&button)
    }

    #[inline]
    pub fn is_button_release(&self, button: MouseButton) -> bool {
        self.releases.contains(&button)
    }

    #[inline]
    pub fn is_button_click(&self, button: MouseButton) -> bool {
        if let Some(v) = self.click_detectors.get(&button) {
            v.clicks() > 0
        } else {
            false
        }
    }

    #[inline]
    pub fn is_button_double_click(&self, button: MouseButton) -> bool {
        if let Some(v) = self.click_detectors.get(&button) {
            v.clicks() > 0 && v.clicks() % 2 == 0
        } else {
            false
        }
    }

    #[inline]
    pub fn position(&self) -> math::Vector2<f32> {
        self.position
    }

    #[inline]
    pub fn movement(&self) -> math::Vector2<f32> {
        self.position - self.last_position
    }

    #[inline]
    pub fn scroll(&self) -> math::Vector2<f32> {
        self.scrol
    }
}

struct ClickDetector {
    last_press_time: Instant,
    last_press_position: math::Vector2<f32>,

    last_click_time: Instant,
    last_click_position: math::Vector2<f32>,

    clicks: u32,
    frame_clicks: u32,

    params: MouseParams,
}

impl ClickDetector {
    pub fn new(params: MouseParams) -> Self {
        ClickDetector {
            last_press_time: Instant::now(),
            last_press_position: math::Vector2::new(0.0, 0.0),

            last_click_time: Instant::now(),
            last_click_position: math::Vector2::new(0.0, 0.0),

            clicks: 0,
            frame_clicks: 0,

            params: params,
        }
    }

    pub fn reset(&mut self) {
        self.clicks = 0;
        self.frame_clicks = 0;
    }

    pub fn advance(&mut self) {
        self.frame_clicks = 0;
    }

    pub fn on_pressed(&mut self, position: math::Vector2<f32>) {
        // Store press down as start of a new potential click.
        let now = Instant::now();

        // If multi-click, checks if within max distance and press timeout of
        // last click, if not, start a new multi-click sequence.
        if self.clicks > 0 {
            if (now - self.last_click_time) > self.params.click_timeout {
                self.reset();
            }

            if (position.distance(self.last_click_position)) > self.params.max_click_distance {
                self.reset();
            }
        }

        self.last_press_time = now;
        self.last_press_position = position;
    }

    pub fn on_released(&mut self, position: math::Vector2<f32>) {
        let now = Instant::now();

        if (now - self.last_press_time) < self.params.press_timeout
            && (position.distance(self.last_press_position)) < self.params.max_press_distance
        {
            self.clicks += 1;
            self.frame_clicks = self.clicks;
            self.last_click_time = now;
            self.last_click_position = position;
        }
    }

    pub fn clicks(&self) -> u32 {
        self.frame_clicks
    }
}
