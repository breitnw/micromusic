
use sdl2::mouse::{MouseState, MouseButton};
use sdl2::pixels::Color;
use sdl2::rect::{Rect, Point};
use sdl2::render::{RenderTarget, Texture, Canvas, TextureCreator};
use sdl2_unifont::renderer::SurfaceRenderer;


//Enum representing button states
enum ButtonState {
    DEFAULT,
    HOVER,
    PRESSED,
}
// Struct representing a button with different states
pub struct Button<'a> {
    rect: Rect,
    texture_default: &'a Texture<'a>,
    texture_hover: &'a Texture<'a>,
    texture_pressed: &'a Texture<'a>,
}

impl<'a> Button<'a> {
    pub fn new(x: i32, y: i32, texture_default: &'a Texture<'a>, texture_hover: &'a Texture<'a>, texture_pressed: &'a Texture<'a>) -> Button<'a> {
        let texture_query = texture_default.query();
        Button {
            texture_default,
            texture_hover,
            texture_pressed,
            rect: Rect::new(x, y, 
                texture_query.width,
                texture_query.height
            ),
        }
    }

    fn get_state(&self, mouse_state: MouseState) -> ButtonState {
        let mouse_pos = Point::new(mouse_state.x(), mouse_state.y());
        if self.rect.contains_point(mouse_pos) {
            if mouse_state.is_mouse_button_pressed(MouseButton::Left) {
                return ButtonState::PRESSED;
            }
            return ButtonState::HOVER;
        }
        return ButtonState::DEFAULT;
    }
    
    pub fn render<T: sdl2::render::RenderTarget>(&self, canvas: &mut Canvas<T>, mouse_state: MouseState) -> std::result::Result<(), String> {
        let tex = match self.get_state(mouse_state) {
            ButtonState::DEFAULT => &self.texture_default,
            ButtonState::HOVER => &self.texture_hover,
            ButtonState::PRESSED => &self.texture_pressed,
        };
        canvas.copy(tex, None, self.rect)?;
        Ok(())
    }

    pub fn is_hovering(&self, mouse_x: i32, mouse_y: i32) -> bool {
        self.rect.contains_point(Point::new(mouse_x, mouse_y))
    }
}


/// Scale the window so it appears the same on high-DPI displays, works fine for now 
/// Based on https://discourse.libsdl.org/t/high-dpi-mode/34411/2
pub fn update_canvas_scale<T: RenderTarget>(canvas: &mut Canvas<T>, window_width: u32, window_height: u32) {
    let (w, h) = canvas.output_size().unwrap();
    canvas.set_scale((w / window_width) as f32, (h / window_height) as f32).unwrap();
}

/// Converts a string to a texture
pub fn text_to_texture<'a, T>(text: &str, texture_creator: &'a TextureCreator<T>) -> Texture<'a> {
    let text_renderer = SurfaceRenderer::new(Color::WHITE, Color::BLACK);
    let text_surface = text_renderer.draw(text).unwrap();
    text_surface.as_texture(texture_creator).unwrap()
}

/// Copies a texture to a canvas without scaling it
pub fn copy_unscaled<T: RenderTarget>(texture: &Texture, x: i32, y: i32, canvas: &mut Canvas<T>) -> std::result::Result<(), String> {
    let query = texture.query();
    canvas.copy(texture, None, Rect::new(x, y, query.width, query.height))?;
    Ok(())
}