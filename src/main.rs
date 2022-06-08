
// FRONT BURNER
// TODO: Add screen for when nothing is playing
// TODO: Only re-render info text every frame, not album art
// TODO: Use a smaller drag icon instead of making the whole window draggable (⠿, maybe sideways)
// TODO: Convert player state to an enum
// TODO: Buttons for skipping forward/back, play/pause, favoriting songs
// TODO: Minimize and close buttons

// CRASHES
// TODO: Probably panic the whole program when the secondary thread panics
// TODO: Fix crash when there's an emoji in song title

// FIXES
// TODO: Flickering when drag begins (use hit test event instead of window moved?) and with trackpad
// TODO: RWops might be done incorrectly, probably don't need to make a new texture every update
// TODO: Title clipping in on songs with short names
// TODO: fix occasional flickering (this should fix itself if album art isn't re-rendered)

// BACK BURNER
// TODO: Make window resizable
// TODO: Dynamically update the draggable areas by syncing with hit_test.c
// TODO: Add anti aliasing
// TODO: Make a third now playing struct for basic song data, embed it in the PlayerData struct, and figure out how to deserialize it


use std::time::{Duration, Instant};
use std::sync::mpsc;
use std::thread;

use sdl2;
use sdl2::libc::c_int;
use sdl2::pixels::Color;
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::rect::{Rect, Point};
use sdl2::render::{ TextureCreator, Texture, BlendMode, Canvas, RenderTarget };
use sdl2_unifont::renderer::SurfaceRenderer;

use sdl2::sys::{SDL_SetWindowHitTest, SDL_Window, SDL_Point, SDL_HitTestResult};
use std::ffi::c_void;

mod player_data;
use player_data::{PlayerData, SongData};
mod osascript_requests;
use osascript_requests::JXACommand;


// PRIMARY THREAD: Renders a SDL2 interface for users to interact with the application
fn main() {
    // Set up a MPSC channel to send player data between threads
    let (tx, rx) = mpsc::channel();

    // Spawn a secondary thread to periodically gather information on the current song and send it to the main thread
    osascript_requests::send_player_data_loop(tx.clone());

    // Initialize SDL
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    // Sizes for the window and canvas
    const ARTWORK_SIZE: u32 = 200;
    const INFO_AREA_HEIGHT: u32 = 40;
    const INFO_PADDING: u32 = 10;

    const WINDOW_WIDTH: u32 = ARTWORK_SIZE;
    const WINDOW_HEIGHT: u32 = ARTWORK_SIZE + INFO_AREA_HEIGHT;
    let window_rect = Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT);

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
    unsafe {
        SDL_SetWindowHitTest(
            window.raw(),
            Some(hitTest),
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

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    update_canvas_scale(&mut canvas, WINDOW_WIDTH, WINDOW_HEIGHT);
    canvas.clear();
    canvas.present();

    // State variables for the rendering loop
    let mut player_and_song_data: Option<(SongData, PlayerData)> = None;
    let mut last_snapshot_time = Instant::now();
    let mut info_scroll_pos: i32 = 0;
    const INFO_SPACING: i32 = 50;

    // A timer used to tell whether the window is currently being moved, decremented every frame
    // Necessary because WindowEvent::Moved is only sent every three frames
    let mut move_detection_timer = 0;

    // This code will run every frame
    'running: loop {
        // Input
        let window_pos = {
            let (x, y) = canvas.window().position();
            Point::new(x, y)
        };
        let mouse_pos_absolute = unsafe {
            let (mut x, mut y): (c_int, c_int) = (0, 0);
            sdl2::sys::SDL_GetGlobalMouseState(&mut x, &mut y); 
            Point::new(x, y)
        };
        let mouse_pos_relative = mouse_pos_absolute - window_pos;
        let window_input_focus = &canvas.window().window_flags() & 512 == 512; // input focus: 512, mouse focus: 1024

        // Reduce the move detection timer by 1
        move_detection_timer = 0.max(move_detection_timer - 1);
        
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                },
                Event::MouseButtonDown {x, y, which, .. } => {
                    if which == 0 { 
                        // TODO: Make buttons for this instead
                        let command = {
                            if x < (WINDOW_WIDTH / 3) as i32 { JXACommand::BackTrack }
                            else if x < (WINDOW_WIDTH * 2 / 3) as i32 { JXACommand::PlayPause }
                            else { JXACommand::NextTrack }
                        };
                        osascript_requests::run_command(command, tx.clone()); 
                    }
                }
                Event::Window { win_event, .. } => {
                    match win_event {
                        WindowEvent::Moved {..} => {
                            move_detection_timer = 3;
                            // Update the canvas scale in case the user drags the window to a different monitor
                            update_canvas_scale(&mut canvas, WINDOW_WIDTH, WINDOW_HEIGHT) 
                        },
                        _ => {}
                    }
                    // TODO: Add event to update canvas rect / dimension vars when canvas is resized
                }
                _ => {}
            }
        }
        
        if let Ok(pl_data) = rx.try_recv() {
            player_and_song_data = match pl_data {
                Some(pl_data_unwrapped) => { 
                    // Since it includes the full image data, this clone is probably pretty significant and may be causing the lag
                    Some((SongData::new(&pl_data_unwrapped, &texture_creator).unwrap(), pl_data_unwrapped))
                }
                None => None
            };
            last_snapshot_time = Instant::now();
        }

        canvas.set_draw_color(Color::BLACK);
        canvas.set_blend_mode(BlendMode::None);
        canvas.clear();

        if let Some((u_song_data, u_player_data)) = &player_and_song_data {
            let info_tex = u_song_data.info_texture();
            let art_tex = u_song_data.artwork_texture();
            let info_qry = u_song_data.info_texture_query();
            //let art_qry = u_song_data.artwork_texture_query();
            
            if u_player_data.player_state == "playing" {
                info_scroll_pos -= 1; 
                info_scroll_pos %= info_qry.width as i32 + INFO_SPACING;
            }

            canvas.copy(&art_tex, None, Rect::new(
                0,
                0,
                ARTWORK_SIZE,
                ARTWORK_SIZE,
            )).unwrap();

            canvas.copy(info_tex, None, Rect::new(
                info_scroll_pos, 
                (ARTWORK_SIZE + INFO_PADDING) as i32, 
                info_qry.width, 
                info_qry.height
            )).unwrap();

            canvas.copy(info_tex, None, Rect::new(
                info_scroll_pos + info_qry.width as i32 + INFO_SPACING, 
                (ARTWORK_SIZE + INFO_PADDING) as i32, 
                info_qry.width, 
                info_qry.height
            )).unwrap();

            if window_input_focus && window_rect.contains_point(mouse_pos_relative) || move_detection_timer > 0 {
                // Darken the cover art
                canvas.set_blend_mode(BlendMode::Mod);
                canvas.set_draw_color(Color::RGB( 150, 150, 150));
                canvas.fill_rect(Rect::new(0, 0, ARTWORK_SIZE, ARTWORK_SIZE)).unwrap();

                // Draw a progress bar
                canvas.set_blend_mode(BlendMode::Add);
                canvas.set_draw_color(Color::RGB(100, 100, 100));

                let percent_elapsed = (u_player_data.player_pos 
                    + if u_player_data.player_state == "playing" { last_snapshot_time.elapsed().as_secs_f64() } else { 0. })
                    / u_player_data.song_length;

                canvas.draw_line(
                    Point::new(0, ARTWORK_SIZE as i32 - 1), 
                    Point::new(
                        (ARTWORK_SIZE as f64 * percent_elapsed) as i32, 
                        ARTWORK_SIZE as i32 - 1
                    )
                ).unwrap();
            }
        }
        
        //Draw a border
        canvas.set_blend_mode(BlendMode::Mod);
        canvas.set_draw_color(Color::RGB(200, 200, 200));
        canvas.draw_rect(Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT)).unwrap();
        canvas.set_blend_mode(BlendMode::Add);
        canvas.set_draw_color(Color::RGB(30, 30, 30));
        canvas.draw_rect(Rect::new(1, 1, WINDOW_WIDTH-2, WINDOW_HEIGHT-2)).unwrap();


        //Present the canvas
        canvas.present();
        thread::sleep(Duration::from_nanos(1_000_000_000u64 / 30));
    }
}

// Scale the window so it appears the same on high-DPI displays, works fine for now 
// Based on https://discourse.libsdl.org/t/high-dpi-mode/34411/2
fn update_canvas_scale<T: RenderTarget>(canvas: &mut Canvas<T>, window_width: u32, window_height: u32) {
    let (w, h) = canvas.output_size().unwrap();
    canvas.set_scale((w / window_width) as f32, (h / window_height) as f32).unwrap();
}


fn text_to_texture<'a, T>(text: &str, texture_creator: &'a TextureCreator<T>) -> Texture<'a> {
    let text_renderer = SurfaceRenderer::new(Color::WHITE, Color::BLACK);
    let text_surface = text_renderer.draw(text).unwrap();
    text_surface.as_texture(texture_creator).unwrap()
}
