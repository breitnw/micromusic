
// An implementation of MouseState that allows for global and relative states
// TODO: Probably move this to a separate file
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct MouseState {
    mouse_state: u32,
    x: i32,
    y: i32,
}

#[allow(dead_code)]
impl MouseState {
    /// Gets the state in the same way as sdl2::mouse:MouseState::new
    pub fn get_state(_e: &sdl2::EventPump) -> MouseState {
        let mut x = 0;
        let mut y = 0;
        let mouse_state: u32 = unsafe { sdl2::sys::SDL_GetMouseState(&mut x, &mut y) };
        MouseState {
            mouse_state, x, y
        }
    }
    /// Gets the global mouse state using SDL_GetGlobalMouseState
    pub fn get_global_state(_e: &sdl2::EventPump) -> MouseState {
        let mut x = 0;
        let mut y = 0;
        let mouse_state: u32 = unsafe { sdl2::sys::SDL_GetGlobalMouseState(&mut x, &mut y) };
        MouseState {
            mouse_state, x, y
        }
    }
    /// Gets the global mouse state, but relative to the window's position
    pub fn get_relative_state(_e: &sdl2::EventPump, window: &sdl2::video::Window) -> MouseState {
        let mut x = 0;
        let mut y = 0;
        let mouse_state: u32 = unsafe { sdl2::sys::SDL_GetGlobalMouseState(&mut x, &mut y) };
        let window_pos = window.position();
        MouseState {
            mouse_state,
            x: x - window_pos.0,
            y: y - window_pos.1,
        }
    }
    pub fn is_mouse_button_pressed(&self, mouse_button: sdl2::mouse::MouseButton) -> bool {
        let mask = 1 << ((mouse_button as u32) - 1);
        self.mouse_state & mask != 0
    }
    pub fn x(&self) -> i32 { self.x }
    pub fn y(&self) -> i32 { self.y }
}
