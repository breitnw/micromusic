use sdl2::image::LoadTexture;
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, RenderTarget, Texture, TextureCreator};
use sdl2_unifont::renderer::SurfaceRenderer;

use image::{self, imageops};

use hex::FromHex;

pub mod mouse;
use mouse::MouseState;

//Enum representing button states
enum ButtonState {
    Default,
    Hover,
    Pressed,
}
// Struct representing a button with different states
pub struct Button<'a> {
    pub collision_rect: Rect,
    render_rect: Rect,
    pub active: bool,
    texture_default: &'a Texture<'a>,
    texture_hover: &'a Texture<'a>,
    texture_pressed: &'a Texture<'a>,
}

impl<'a> Button<'a> {
    pub fn new(
        x: i32,
        y: i32,
        texture_default: &'a Texture<'a>,
        texture_hover: &'a Texture<'a>,
        texture_pressed: &'a Texture<'a>,
    ) -> Button<'a> {
        let texture_query = texture_default.query();
        const BUTTON_COLLISION_MARGIN: u8 = 2;
        Button {
            collision_rect: Rect::new(
                x - BUTTON_COLLISION_MARGIN as i32,
                y - BUTTON_COLLISION_MARGIN as i32,
                texture_query.width + BUTTON_COLLISION_MARGIN as u32 * 2,
                texture_query.height + BUTTON_COLLISION_MARGIN as u32 * 2,
            ),
            render_rect: Rect::new(x, y, texture_query.width, texture_query.height),
            active: true,
            texture_default,
            texture_hover,
            texture_pressed,
        }
    }

    fn get_state(&self, mouse_state: MouseState) -> ButtonState {
        let mouse_pos = Point::new(mouse_state.x(), mouse_state.y());
        if self.collision_rect.contains_point(mouse_pos) {
            if mouse_state.is_mouse_button_pressed(MouseButton::Left) {
                return ButtonState::Pressed;
            }
            return ButtonState::Hover;
        }
        return ButtonState::Default;
    }

    pub fn render<T: sdl2::render::RenderTarget>(
        &self,
        canvas: &mut Canvas<T>,
        mouse_state: MouseState,
    ) -> std::result::Result<(), String> {
        if self.active {
            let tex = match self.get_state(mouse_state) {
                ButtonState::Default => &self.texture_default,
                ButtonState::Hover => &self.texture_hover,
                ButtonState::Pressed => &self.texture_pressed,
            };
            canvas.copy(tex, None, self.render_rect)?;
        }
        Ok(())
    }

    pub fn is_hovering(&self, mouse_x: i32, mouse_y: i32) -> bool {
        self.active
            && self
                .collision_rect
                .contains_point(Point::new(mouse_x, mouse_y))
    }
}

/// Scale the window so it appears the same on high-DPI displays, works fine for now
/// Based on https://discourse.libsdl.org/t/high-dpi-mode/34411/2
pub fn update_canvas_scale<T: RenderTarget>(
    canvas: &mut Canvas<T>,
    window_width: u32,
    window_height: u32,
) {
    let (w, h) = canvas.output_size().unwrap();
    canvas
        .set_scale((w / window_width) as f32, (h / window_height) as f32)
        .unwrap();
}

/// Converts a string to a texture
pub fn text_to_texture<'a, T>(
    text: &str,
    texture_creator: &'a TextureCreator<T>,
    foreground_color: Color,
    background_color: Color,
) -> Texture<'a> {
    let text_renderer = SurfaceRenderer::new(foreground_color, background_color);
    let text_surface = text_renderer.draw(text).unwrap();
    text_surface.as_texture(texture_creator).unwrap()
}

/// Copies a texture to a canvas without scaling it
pub fn copy_unscaled<T: RenderTarget>(
    texture: &Texture,
    x: i32,
    y: i32,
    canvas: &mut Canvas<T>,
) -> std::result::Result<(), String> {
    let query = texture.query();
    canvas.copy(texture, None, Rect::new(x, y, query.width, query.height))?;
    Ok(())
}

pub fn raw_to_cached_image<'a>(raw_data: &str, size: (u32, u32), cache_path: &str) -> std::result::Result<(), hex::FromHexError> {
    // Load the raw bytes
    let bytes = Vec::from_hex(&raw_data[8..raw_data.len() - 2])?;

    // If the caller specified a target image size, resize the image to that size
    let image = image::load_from_memory(&bytes)
        .unwrap()
        .resize(size.0, size.1, imageops::FilterType::Nearest);

    // TODO: shouldn't unwrap this
    image.save_with_format(cache_path, image::ImageFormat::Png).unwrap();

    Ok(())
} 

pub fn raw_to_texture<'a, 'b, T>(
    raw_data: &'a str, 
    texture_creator: &'b TextureCreator<T>, 
) -> std::result::Result<Texture<'b>, hex::FromHexError> {
    // Use linear filtering when creating the texture
    sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "linear");

    // Load the raw bytes
    let bytes = Vec::from_hex(&raw_data[8..raw_data.len() - 2])?;

    // Load the texture
    let artwork_texture = texture_creator.load_texture_bytes(&bytes).unwrap();

    // Reset filtering to nearest
    sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "nearest");

    Ok(artwork_texture)
}

