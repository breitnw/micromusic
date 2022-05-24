//TODO: Remove the title bar and make the window itself draggable
//TODO: This shit also breaks if at any point there's no song playing fsr
//TODO: Better canvas scaling on regular/high dpi displays, see link in comment
//TODO: Fix title clipping in on songs with short names

use std::time::Duration;
use std::sync::mpsc;
use std::thread;

use osascript;

use sdl2;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::render::{TextureCreator, Texture};
use sdl2_unifont::renderer::SurfaceRenderer;

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
    let mut window = video_subsystem.window("music app", 298, 40)
        .position_centered()
        .allow_highdpi()
        .build()
        .unwrap();
    window.raise();
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
        
        if let Ok(raw_data) = rx.try_recv() {
            song_data = Some(SongData::new(raw_data, &texture_creator));
        }

        //println!("{} - {}\n{}", artist, name, album);

        canvas.set_draw_color(Color::BLACK);
        canvas.clear();

        if let Some(u_song_data) = &song_data {
            let qry = u_song_data.texture_query();
            let tex = u_song_data.texture();

            x -= 1;
            x %= qry.width as i32 + SPACING;

            canvas.copy(tex, None, Rect::new(
                x, 
                10, 
                qry.width, 
                qry.height
            )).unwrap();

            canvas.copy(tex, None, Rect::new(
                x + qry.width as i32 + SPACING, 
                10, 
                qry.width, 
                qry.height
            )).unwrap();
        }

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
fn start_osascript(tx: mpsc::Sender<RawSongData>) {
    let script = osascript::JavaScript::new("
        var App = Application('Music');
        var track = App.currentTrack
        return [track.name(), track.artist(), track.album()];
    ");
    thread::spawn(move || {
        loop {
            let rv: Vec<String> = script.execute().unwrap();
            let song_data = RawSongData::new(
                rv[0].to_owned(),
                rv[1].to_owned(),
                rv[2].to_owned(),
            );
            tx.send(song_data).expect("Couldn't send song data through the channel");
            thread::sleep(Duration::from_secs(2));
        }
    });
}
