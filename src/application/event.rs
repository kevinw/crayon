//! Responsible for converting window messages to input state and internal events.

use glutin;
pub use glutin::MouseButton;
pub use glutin::VirtualKeyCode as KeyboardButton;

use math;

/// The status of application.
#[derive(Debug, Clone, Copy)]
pub enum ApplicationEvent {
    /// The window has been woken up by another thread.
    Awakened,
    /// The window has been resumed.
    Resumed,
    /// The window has been suspended.
    Suspended,
    /// The window has been closed.
    Closed,
    /// The window gained focus of user input.
    GainFocus,
    /// The window lost focus of user input.
    LostFocus,
    /// The size of window has changed.
    Resized(u32, u32),
    /// The position of window has changed.
    Moved(u32, u32),
}

/// Input device event, supports mouse and keyboard only.
#[derive(Debug, Clone, Copy)]
pub enum InputDeviceEvent {
    /// The cursor has moved on the window.
    /// The parameter are the (x, y) coords in pixels relative to the top-left
    /// corner of th window.
    MouseMoved { position: (f32, f32) },
    /// Pressed event on mouse has been received.
    MousePressed { button: MouseButton },
    /// Released event from mouse has been received.
    MouseReleased { button: MouseButton },
    /// A mouse wheel movement or touchpad scroll occurred.
    MouseWheel { delta: (f32, f32) },

    /// Pressed event on keyboard has been received.
    KeyboardPressed { key: KeyboardButton },
    /// Released event from keyboard has been received.
    KeyboardReleased { key: KeyboardButton },
    /// Received a unicode character.
    ReceivedCharacter { character: char },

    /// Represent touch event.
    ///
    /// Every time user touches screen new Start event with some finger id is generated. When
    /// the finger is removed from the screen End event with same id is generated.
    ///
    /// For every id there will be at least 2 events with phases Start and End (or Cancel).
    /// There may be 0 or more Move events.
    ///
    /// Depending on platform implementation id may or may not be reused by system after End event.
    Touch(TouchEvent),
}

/// The enumerations of all events that come from various kinds of user input.
#[derive(Debug, Clone, Copy)]
pub enum Event {
    Application(ApplicationEvent),
    InputDevice(InputDeviceEvent),
}

pub(crate) fn from_event(source: glutin::Event, dimensions: math::Vector2<u32>) -> Option<Event> {
    match source {
        glutin::Event::WindowEvent { event, .. } => from_window_event(&event, dimensions),

        glutin::Event::Awakened => Some(Event::Application(ApplicationEvent::Awakened)),

        glutin::Event::Suspended(v) => if v {
            Some(Event::Application(ApplicationEvent::Suspended))
        } else {
            Some(Event::Application(ApplicationEvent::Resumed))
        },

        glutin::Event::DeviceEvent { .. } => None,
    }
}

fn from_window_event(
    source: &glutin::WindowEvent,
    dimensions: math::Vector2<u32>,
) -> Option<Event> {
    match *source {
        glutin::WindowEvent::CloseRequested => Some(Event::Application(ApplicationEvent::Closed)),

        glutin::WindowEvent::Focused(v) => if v {
            Some(Event::Application(ApplicationEvent::GainFocus))
        } else {
            Some(Event::Application(ApplicationEvent::LostFocus))
        },

        glutin::WindowEvent::CursorMoved { position, .. } => {
            Some(Event::InputDevice(InputDeviceEvent::MouseMoved {
                position: (position.x as f32, dimensions.y as f32 - position.y as f32),
            }))
        }

        glutin::WindowEvent::MouseWheel { delta, .. } => match delta {
            glutin::MouseScrollDelta::LineDelta(x, y) => {
                Some(Event::InputDevice(InputDeviceEvent::MouseWheel {
                    delta: (x as f32, y as f32),
                }))
            }
            glutin::MouseScrollDelta::PixelDelta(pos) => {
                Some(Event::InputDevice(InputDeviceEvent::MouseWheel {
                    delta: (pos.x as f32, pos.y as f32),
                }))
            }
        },

        glutin::WindowEvent::MouseInput {
            state: glutin::ElementState::Pressed,
            button,
            ..
        } => Some(Event::InputDevice(InputDeviceEvent::MousePressed {
            button,
        })),

        glutin::WindowEvent::MouseInput {
            state: glutin::ElementState::Released,
            button,
            ..
        } => Some(Event::InputDevice(InputDeviceEvent::MouseReleased {
            button,
        })),

        glutin::WindowEvent::KeyboardInput {
            input:
                glutin::KeyboardInput {
                    state: glutin::ElementState::Pressed,
                    virtual_keycode: Some(key),
                    ..
                },
            ..
        } => Some(Event::InputDevice(InputDeviceEvent::KeyboardPressed {
            key,
        })),

        glutin::WindowEvent::KeyboardInput {
            input:
                glutin::KeyboardInput {
                    state: glutin::ElementState::Released,
                    virtual_keycode: Some(key),
                    ..
                },
            ..
        } => Some(Event::InputDevice(InputDeviceEvent::KeyboardReleased {
            key,
        })),

        glutin::WindowEvent::ReceivedCharacter(character) => Some(Event::InputDevice(
            InputDeviceEvent::ReceivedCharacter { character },
        )),

        glutin::WindowEvent::Touch(touch) => {
            let evt = TouchEvent {
                id: touch.id as u8,
                state: from_touch_state(touch.phase),
                position: (touch.location.x as f32, touch.location.y as f32).into(),
            };

            Some(Event::InputDevice(InputDeviceEvent::Touch(evt)))
        }

        _ => None,
    }
}

fn from_touch_state(state: glutin::TouchPhase) -> TouchState {
    match state {
        glutin::TouchPhase::Started => TouchState::Start,
        glutin::TouchPhase::Moved => TouchState::Move,
        glutin::TouchPhase::Ended => TouchState::End,
        glutin::TouchPhase::Cancelled => TouchState::Cancel,
    }
}

/// Describes touch-screen input state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TouchState {
    Start,
    Move,
    End,
    Cancel,
}

#[derive(Debug, Clone, Copy)]
pub struct TouchEvent {
    pub id: u8,
    pub state: TouchState,
    pub position: math::Vector2<f32>,
}

impl Default for TouchEvent {
    fn default() -> Self {
        TouchEvent {
            id: 0,
            state: TouchState::End,
            position: math::Vector2::new(0.0, 0.0),
        }
    }
}
