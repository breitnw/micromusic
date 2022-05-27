
// Front burner
// TODO: Figure out how to handle errors that arise when no song is playing
// TODO: Better canvas scaling on regular/high dpi displays, see link in comment
// TODO: Only re-render info text every frame, not album art
// TODO: Potentially save on resources by reducing framerate

// Back burner
// TODO: Fix title clipping in on songs with short names
// TODO: Ideally propagate errors when getting album artwork instead of unwrapping
// TODO: Make window resizable
// TODO: Dynamically update the draggable areas by syncing with hit_test.c

use std::time::Duration;
use std::sync::mpsc;
use std::thread;

use osascript;

use sdl2;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::render::{ TextureCreator, Texture, BlendMode };
use sdl2_unifont::renderer::SurfaceRenderer;

use sdl2::sys::{SDL_SetWindowHitTest, SDL_HitTest, SDL_Window, SDL_Point, SDL_HitTestResult};
use std::ffi::c_void;

mod song_info;
use song_info::{RawSongData, SongData};


// PRIMARY THREAD: Renders a SDL2 interface for users to interact with the application
fn main() {
    // Set up a MPSC channel to send song data between threads
    let (tx, rx) = mpsc::channel();

    // Spawn a secondary thread 
    start_osascript(tx);

    // Initialize SDL
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    // Create the window and canvas
    const ARTWORK_SIZE: u32 = 200;
    const INFO_AREA_HEIGHT: u32 = 40;
    const INFO_PADDING: u32 = 10;

    const WINDOW_WIDTH: u32 = ARTWORK_SIZE;
    const WINDOW_HEIGHT: u32 = ARTWORK_SIZE + INFO_AREA_HEIGHT;

    //Create the window
    let mut window = video_subsystem.window("music app", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .allow_highdpi()
        .borderless()
        .build()
        .unwrap();
    window.raise();

    // Make the window draggable
    extern { 
        pub fn hitTest(window: *mut SDL_Window, pt: *const SDL_Point, data: *mut c_void) -> SDL_HitTestResult;
    }
    let wrapped_callback: SDL_HitTest = Some(hitTest);
    unsafe {
        SDL_SetWindowHitTest(
            window.raw(),
            wrapped_callback,
            1 as *mut sdl2::libc::c_void,
        );
    }

    //Create a canvas from the window
    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .unwrap();

    // Get the canvas's texture creator
    let texture_creator = canvas.texture_creator();

    // Find a better solution for this later - https://discourse.libsdl.org/t/high-dpi-mode/34411/2
    canvas.set_scale(2., 2.).unwrap();
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();
    
    let mut song_data: Option<SongData> = None;

    let mut x: i32 = 0;
    const SPACING: i32 = 50;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                },
                _ => {}
            }
        }
        
        if let Ok(wrapped_data) = rx.try_recv() {
            match wrapped_data {
                Some(raw_data) => { song_data = Some(SongData::new(raw_data, &texture_creator)); }
                None => { song_data = None }
            }
        }

        canvas.set_draw_color(Color::BLACK);
        canvas.set_blend_mode(BlendMode::None);
        canvas.clear();

        if let Some(u_song_data) = &song_data {
            let info_tex = u_song_data.info_texture();
            let art_tex = u_song_data.artwork_texture();
            let info_qry = u_song_data.info_texture_query();
            //let art_qry = u_song_data.artwork_texture_query();
            
            x -= 1;
            x %= info_qry.width as i32 + SPACING;

            canvas.copy(&art_tex, None, Rect::new(
                0,
                0,
                ARTWORK_SIZE,
                ARTWORK_SIZE,
            )).unwrap();

            canvas.copy(info_tex, None, Rect::new(
                x, 
                (ARTWORK_SIZE + INFO_PADDING) as i32, 
                info_qry.width, 
                info_qry.height
            )).unwrap();

            canvas.copy(info_tex, None, Rect::new(
                x + info_qry.width as i32 + SPACING, 
                (ARTWORK_SIZE + INFO_PADDING) as i32, 
                info_qry.width, 
                info_qry.height
            )).unwrap();
        }
        
        //Draw a border
        canvas.set_blend_mode(BlendMode::Blend);
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 150));
        canvas.draw_rect(Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT)).unwrap();
        canvas.set_draw_color(Color::RGBA(255, 255, 255, 50));
        canvas.draw_rect(Rect::new(1, 1, WINDOW_WIDTH-2, WINDOW_HEIGHT-2)).unwrap();

        //Present the canvas
        canvas.present();
        thread::sleep(Duration::from_nanos(1_000_000_000u64 / 30));
    }
}


fn text_to_texture<'a, T>(text: &str, texture_creator: &'a TextureCreator<T>) -> Texture<'a> {
    let text_renderer = SurfaceRenderer::new(Color::WHITE, Color::BLACK);
    let text_surface = text_renderer.draw(text).unwrap();
    text_surface.as_texture(texture_creator).unwrap()
}


// SECONDARY THREAD: Periodically runs a JXA script to gather information on the current song
fn start_osascript(tx: mpsc::Sender<Option<RawSongData>>) {
    let script = osascript::JavaScript::new("
        var App = Application('Music');
        var track = App.currentTrack
        var artwork = track.artworks[0];
        return [track.name(), track.artist(), track.album(), artwork.rawData()];
    ");
    thread::spawn(move || {
        loop {
            if let Ok::<Vec<String>, _>(rv) = script.execute() {
                let song_data = RawSongData::new(
                    rv[0].to_owned(),
                    rv[1].to_owned(),
                    rv[2].to_owned(),
                    rv[3].to_owned(),
                );
                tx.send(Some(song_data)).expect("Couldn't send song data through the channel");
            } 
            else {
                tx.send(None).expect("Couldn't send song data through the channel");
                println!("Error receiving data from Apple Music.")
            }
            thread::sleep(Duration::from_secs(3));
        }
    });
}
