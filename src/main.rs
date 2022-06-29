
// FRONT BURNER
// TODO: Function to set drag areas
// Maybe a system where you can add draggable rects and then non-draggable rects (buttons, etc) on top of them
// TODO: Fix icons by making them clickable
// TODO: Buttons for skipping forward/back, play/pause, favoriting tracks
// TODO: Use a smaller drag icon instead of making the whole window draggable (⠿, maybe sideways)
// TODO: Convert player state to an enum

// CRASHES
// TODO: Probably panic the whole program when the secondary thread panics
// TODO: Fix crash when there's an emoji in track title

// FIXES
// TODO: Flickering when drag begins (use hit test event instead of window moved?) and with trackpad
// TODO: RWops might be done incorrectly, probably don't need to make a new texture every update
// TODO: Title clipping in on tracks with short names
// TODO: Antialiasing issues moving back and forth between displays (try regenerating texture?)

// BACK BURNER
// TODO: Make window resizable
// TODO: Add screen for when nothing is playing
// TODO: Add anti aliasing
// TODO: Make a third now playing struct for basic track data, embed it in the PlayerData struct, and figure out how to deserialize it
// TODO: Only re-render info text every frame, not album art


use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::mpsc;
use std::thread;

use rust_embed::RustEmbed;

use sdl2;
use sdl2::image::LoadTexture;
use sdl2::libc::c_int;
use sdl2::pixels::Color;
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::rect::{Rect, Point};
use sdl2::render::{ TextureCreator, Texture, BlendMode };

use sdl2::sys::{SDL_SetWindowHitTest, SDL_Window, SDL_Point, SDL_HitTestResult, SDL_GetGlobalMouseState};
use std::ffi::c_void;

mod player_data;
use player_data::{PlayerData, TrackData};
mod osascript_requests;
use osascript_requests::JXACommand;
mod engine;
use engine::Button;


// PRIMARY THREAD: Renders a SDL2 interface for users to interact with the application
fn main() {
    // Set up a MPSC channel to send player data between threads
    let (tx, rx) = mpsc::channel();

    // Spawn a secondary thread to periodically gather information on the current track and send it to the main thread
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
    let mut window = video_subsystem.window("micromusic", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .allow_highdpi()
        .borderless()
        .build()
        .unwrap();
    window.raise();

    // Make the window draggable
    extern { 
        // pub fn init();
        pub fn hitTest(window: *mut SDL_Window, pt: *const SDL_Point, data: *mut c_void) -> SDL_HitTestResult;
    }
    unsafe {
        // init();
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

    //Set up and present the canvas
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    engine::update_canvas_scale(&mut canvas, WINDOW_WIDTH, WINDOW_HEIGHT);
    canvas.clear();
    canvas.present();


    // Create the icon textures and load them into a dictionary
    // TODO: Maybe move to engine
    // TODO: Only load files with .png extension (not .DS_Store if it exists)
    #[derive(RustEmbed)]
    #[folder = "assets/icons/"]
    struct IconAsset;

    fn load_icons<'a, T: 'a>(color_mod: u8, blend_mode: BlendMode, texture_creator: &'a TextureCreator<T>) -> HashMap<String, Texture<'a>> {
        IconAsset::iter().map(|file_path| {
            let mut tex = texture_creator.load_texture_bytes(&IconAsset::get(&file_path).unwrap().data)
                .unwrap_or_else(|_| panic!("Unable to load {}", &file_path));
            tex.set_color_mod(color_mod, color_mod, color_mod);
            tex.set_blend_mode(blend_mode);
            (file_path.to_string(), tex)
        }).collect()
    }

    const ICON_COLOR_MOD_DEFAULT: u8 = 100;
    const ICON_COLOR_MOD_HOVER: u8 = 200;
    let icon_textures_default = load_icons(ICON_COLOR_MOD_DEFAULT, BlendMode::Add, &texture_creator);
    let icon_textures_hover = load_icons(ICON_COLOR_MOD_HOVER, BlendMode::Add, &texture_creator);
    

    // State variables for the rendering loop
    let mut player_and_track_data: Option<(TrackData, PlayerData)> = None;
    let mut last_snapshot_time = Instant::now();
    let mut info_scroll_pos: i32 = 0;
    const INFO_SPACING: i32 = 50;

    // A timer used to tell whether the window is currently being moved, decremented every frame
    // Necessary because WindowEvent::Moved is only sent every three frames
    let mut move_detection_timer = 0;


    // A map of all of the buttons on the screen
    // TODO: Probably use a macro or something instead of this gross hashmap
    // TODO: Make a simpler function for constructing buttons
    let mut buttons: HashMap<&'static str, Button> = HashMap::from([
        ("heart_empty", Button::new( 5, 5,
            &icon_textures_default["heart_empty.png"],
            &icon_textures_hover["heart_empty.png"],
            &icon_textures_hover["heart_empty.png"],
        )),
        ("heart_filled", Button::new( 5, 5,
            &icon_textures_default["heart_filled.png"],
            &icon_textures_hover["heart_filled.png"],
            &icon_textures_hover["heart_filled.png"],
        )),
        ("minimize", Button::new( ARTWORK_SIZE as i32 - 30, 5,
            &icon_textures_default["minimize.png"],
            &icon_textures_hover["minimize.png"],
            &icon_textures_hover["minimize.png"],
        )),
        ("close", Button::new( ARTWORK_SIZE as i32 - 16, 5,
            &icon_textures_default["close.png"],
            &icon_textures_hover["close.png"],
            &icon_textures_hover["close.png"],
        )),
        ("pause", Button::new( ARTWORK_SIZE as i32 / 2 - 5, ARTWORK_SIZE as i32 / 2 - 5,
            &icon_textures_default["pause.png"],
            &icon_textures_hover["pause.png"],
            &icon_textures_hover["pause.png"],
        )),
        ("next_track", Button::new( ARTWORK_SIZE as i32 / 2 - 6 + 18, ARTWORK_SIZE as i32 / 2 - 5,
            &icon_textures_default["next_track.png"],
            &icon_textures_hover["next_track.png"],
            &icon_textures_hover["next_track.png"],
        )),
        ("back_track", Button::new( ARTWORK_SIZE as i32 / 2 - 6 - 18, ARTWORK_SIZE as i32 / 2 - 5,
            &icon_textures_default["back_track.png"],
            &icon_textures_hover["back_track.png"],
            &icon_textures_hover["back_track.png"],
        )),
        // ("loop", Button::new( ARTWORK_SIZE as i32 / 2 - 6 + 44, ARTWORK_SIZE as i32 - 20,
        //     &icon_textures_default["loop.png"],
        //     &icon_textures_hover["loop.png"],
        //     &icon_textures_hover["loop.png"],
        // )),
        // ("shuffle", Button::new( ARTWORK_SIZE as i32 / 2 - 6 - 44, ARTWORK_SIZE as i32 - 20,
        //     &icon_textures_default["shuffle.png"],
        //     &icon_textures_hover["shuffle.png"],
        //     &icon_textures_hover["shuffle.png"],
        // )),
    ]);


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
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                },
                Event::MouseButtonUp { x, y, which, .. } => {
                    if which == 0 { 
                        // TODO: Make buttons for this instead
                        if y > ARTWORK_SIZE as i32 {
                            let command = {
                                if x < (WINDOW_WIDTH / 3) as i32 { JXACommand::BackTrack }
                                else if x < (WINDOW_WIDTH * 2 / 3) as i32 { JXACommand::PlayPause }
                                else { JXACommand::NextTrack }
                            };
                            osascript_requests::run_command(command, tx.clone()); 
                        }
                        if buttons["heart_empty"].is_hovering(x, y) { 
                            osascript_requests::run_command(JXACommand::Love, tx.clone()) 
                        } else if buttons["heart_filled"].is_hovering(x, y) { 
                            osascript_requests::run_command(JXACommand::Unlove, tx.clone()) 
                        }
                        // TODO: This doesn't work fsr
                        if buttons["minimize"].is_hovering(x, y) { canvas.window_mut().minimize(); }
                        if buttons["close"].is_hovering(x, y) { break 'running; }
                    }
                }
                Event::Window { win_event, .. } => {
                    match win_event {
                        WindowEvent::Moved {..} => {
                            move_detection_timer = 3;
                            // Update the canvas scale in case the user drags the window to a different monitor
                            engine::update_canvas_scale(&mut canvas, WINDOW_WIDTH, WINDOW_HEIGHT) 
                        },
                        _ => {}
                    }
                    // TODO: Add event to update canvas rect / dimension vars when canvas is resized
                }
                _ => {}
            }
        }
        
        // If the channel has new data in it, update the player and track data on this thread
        if let Ok(pl_data) = rx.try_recv() {
            player_and_track_data = match pl_data {
                Some(pl_data_unwrapped) => { 
                    // Since it includes the filled image data, this clone is probably pretty significant and may be causing the lag
                    Some((TrackData::new(&pl_data_unwrapped, &texture_creator).unwrap(), pl_data_unwrapped))
                }
                None => None
            };
            last_snapshot_time = Instant::now();
        }

        // Clear the canvas for drawing
        canvas.set_draw_color(Color::BLACK);
        canvas.set_blend_mode(BlendMode::None);
        canvas.clear();

        if let Some((u_track_data, u_player_data)) = &player_and_track_data {
            let info_tex = u_track_data.info_texture();
            let info_qry = info_tex.query();
            let art_tex = u_track_data.artwork_texture();
            
            if u_player_data.player_state == "playing" {
                info_scroll_pos -= 1; 
                info_scroll_pos %= info_qry.width as i32 + INFO_SPACING;
            }
            
            // Draw the album art
            canvas.copy(art_tex, None, Rect::new(0,0, 
                ARTWORK_SIZE, 
                ARTWORK_SIZE
            )).unwrap();

            //Draw the info text, once normally and once shifted to the right for seamless looping
            engine::copy_unscaled(&info_tex, 
                info_scroll_pos, 
                (ARTWORK_SIZE + INFO_PADDING) as i32, 
                &mut canvas
            ).unwrap();
            engine::copy_unscaled(&info_tex, 
                info_scroll_pos + info_qry.width as i32 + INFO_SPACING, 
                (ARTWORK_SIZE + INFO_PADDING) as i32, 
                &mut canvas
            ).unwrap();

            //Draw an overlay if the user is hovering over the window
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
                    / u_player_data.track_length;

                canvas.draw_line(
                    Point::new(0, ARTWORK_SIZE as i32 - 1), 
                    Point::new(
                        (ARTWORK_SIZE as f64 * percent_elapsed) as i32, 
                        ARTWORK_SIZE as i32 - 1
                    )
                ).unwrap();

                // Update button visibility based on new data
                buttons.get_mut("heart_empty").unwrap().set_active(!u_track_data.loved());
                buttons.get_mut("heart_filled").unwrap().set_active(u_track_data.loved());

                // Draw the button icons
                // TODO: Use global mouse state to prevent button sticking after going out of frame
                for button in buttons.values() { button.render(&mut canvas, event_pump.mouse_state()).unwrap(); }
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
