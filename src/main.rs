
// FRONT BURNER
// TODO: build textures asynchronously, evenly distributed over a specified number of threads
// TODO: don't get textures with applescript if they're already cached
// TODO: account for big/little endian?
// TODO: instead of not creating now_playing_resources if not playing, create a dummy value that
// says something like "not playing"

// CRASHES
// TODO: Probably panic the whole program when the secondary thread panics
// TODO: Fix crash when there's an emoji in track title
// TODO: Fix crash when clicking heart button while nothing's playing

// FIXES
// TODO: Title clipping on tracks with short names
// TODO: Antialiasing issues moving back and forth between displays (try regenerating texture or setting hint when changing screens)

// BACK BURNER
// TODO: Make window resizable
// TODO: Add screen for when nothing is playing, make sure to draw overlay buttons
// TODO: Only re-render info text every frame, not album art
// TODO: Automatically get text colors based on image / dark or light theme
// TODO: Remember window position on close


use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;

use rust_embed::RustEmbed;

use sdl2;
use sdl2::image::LoadTexture;
use sdl2::libc::c_int;
use sdl2::pixels::Color;
use sdl2::event::{Event, WindowEvent};
use sdl2::rect::{Rect, Point};
use sdl2::render::{ TextureCreator, Texture, BlendMode };

use sdl2::sys::{SDL_SetWindowHitTest, SDL_Window, SDL_Point, SDL_Rect, SDL_HitTestResult};
use std::ffi::c_void;

mod player_data;
use player_data::PlayerState;
mod album_data;
use album_data::AlbumResources;
mod osascript_requests;
use osascript_requests::JXACommand;
mod engine;
use engine::Button;

use crate::album_data::BaseAlbumResources;
use crate::player_data::NowPlayingResourceCollection;


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

    // Create the window
    let mut window = video_subsystem.window("micromusic", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .allow_highdpi()
        .borderless()
        .build()
        .unwrap();
    window.raise();

    
    // Make the window draggable
    fn raw_heap_rect(x: c_int, y: c_int, w: c_int, h: c_int) -> *const SDL_Rect {
        Box::into_raw(Box::new(SDL_Rect { x, y, w, h } ))
    }

    let add = vec![ raw_heap_rect(0, 0, WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32) ];
    let mut sub = vec![];
    
    #[repr(C)]
    struct HitTestData {
        add: *const *const SDL_Rect,
        add_len: c_int,
        sub: *const *const SDL_Rect,
        sub_len: c_int
    }
    let mut hit_test_data = HitTestData {
        add: add.as_ptr(),
        add_len: add.len() as c_int,
        sub: sub.as_ptr(),
        sub_len: sub.len() as c_int,
    };
    
    extern "C" { 
        pub fn hitTest(window: *mut SDL_Window, pt: *const SDL_Point, data: *mut c_void) -> SDL_HitTestResult;
    }
    unsafe {
        SDL_SetWindowHitTest(
            window.raw(),
            Some(hitTest),
            &mut hit_test_data as *mut _ as *mut c_void,
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
    let mut now_playing_resources: Option<NowPlayingResourceCollection> = None;
    let mut last_snapshot_time = Instant::now();
    let mut info_scroll_pos: i32 = 0;
    const INFO_SPACING: i32 = 50;

    // A boolean that specifies whether the user is currently dragging the window. Set when a window drag event occurs,
    // reset when the mouse button is released
    let mut window_interaction_in_progress = false;

    // Create a map of all of the buttons on the screen
    let button_data = {
        const A_SIZE: i32 = ARTWORK_SIZE as i32;
        [
            ("heart_empty", (5, 5)),
            ("heart_filled", (5, 5)),
            ("minimize", (A_SIZE - 30, 5)),
            ("close", (A_SIZE - 16, 5)),
            ("pause", (A_SIZE / 2 - 5, A_SIZE - 20)),
            ("play", (A_SIZE / 2 - 5, A_SIZE - 20)),
            ("next_track", (A_SIZE / 2 - 6 + 18, A_SIZE - 20)),
            ("back_track", (A_SIZE / 2 - 6 - 18, A_SIZE - 20)),
        ]
    };
    let mut buttons: HashMap<&'static str, Button> = button_data.into_iter()
        .map(|b| (b.0, {
            let filename = String::from(b.0) + ".png";
            Button::new(b.1.0, b.1.1, 
                &icon_textures_default[&filename],
                &icon_textures_hover[&filename],
                &icon_textures_hover[&filename],
            )
        }))
        .collect();
    
    // Spawn a thread to get all album resources from Apple Music
    let mut album_resources: Option<Vec<AlbumResources>> = None;
    let (ar_tx, ar_rx) = mpsc::channel();
    thread::spawn(move || {
        let base_album_resources = BaseAlbumResources::get_all_from_music();
        ar_tx.send(base_album_resources).unwrap();
    });
    

    // This code will run every frame
    'running: loop {

        // Mouse state
        let mouse_state = engine::mouse::MouseState::get_relative_state(&event_pump, canvas.window());
        let window_input_focus = &canvas.window().window_flags() & 512 == 512; // input focus: 512, mouse focus: 1024
        
        // Iterate through the input events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    break 'running;
                },
                Event::MouseButtonUp { x, y, which, .. } => {
                    if which == 0 {
                        // TODO: Make buttons for this instead
                        if buttons["heart_empty"].is_hovering(x, y) { 
                            osascript_requests::run_command(JXACommand::Love, tx.clone()) 
                        } 
                        else if buttons["heart_filled"].is_hovering(x, y) { 
                            osascript_requests::run_command(JXACommand::Unlove, tx.clone()) 
                        }
                        else if buttons["play"].is_hovering(x, y) || buttons["pause"].is_hovering(x, y) {
                            osascript_requests::run_command(JXACommand::PlayPause, tx.clone())
                        }
                        else if buttons["back_track"].is_hovering(x, y) {
                            osascript_requests::run_command(JXACommand::BackTrack, tx.clone())
                        }
                        else if buttons["next_track"].is_hovering(x, y) {
                            osascript_requests::run_command(JXACommand::NextTrack, tx.clone())
                        }
                        else if buttons["minimize"].is_hovering(x, y) { 
                            // TODO: Probably find a better solution, this is janky asf
                            canvas.window_mut().set_bordered(true);
                            canvas.window_mut().minimize(); 
                            canvas.window_mut().set_bordered(false);
                        }
                        else if buttons["close"].is_hovering(x, y) { break 'running; }
                    }
                }
                Event::Window { win_event, .. } => {
                    match win_event {
                        WindowEvent::Moved {..} => {
                            // Update the canvas scale in case the user drags the window to a different monitor
                            engine::update_canvas_scale(&mut canvas, WINDOW_WIDTH, WINDOW_HEIGHT) 
                        },
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // Reset drag_in_progress if the mouse button was just lifted
        if mouse_state.is_mouse_button_pressed(sdl2::mouse::MouseButton::Left) {
            if window_rect.contains_point(Point::new(mouse_state.x(), mouse_state.y())) && window_input_focus {
                window_interaction_in_progress = true;
            }
        } else {
            window_interaction_in_progress = false;
        }

        // If the base album resources are done loading, create and save their artwork textures
        if let Ok(response) = ar_rx.try_recv() {
            album_resources = Some(
                response.into_iter()
                    .map(|r| r.construct_artwork(&texture_creator))
                    .collect()
            );
        }
        
        // If the channel has new data in it, update the player and track data on this thread
        if let Ok(response) = rx.try_recv() {
            if let Some(u_response) = response {
                if let Some(u_np_resources) = now_playing_resources.as_mut() {
                    u_np_resources.update(u_response, &texture_creator);
                } else {
                    now_playing_resources = Some(NowPlayingResourceCollection::build(u_response, &texture_creator));
                }
                
            } else { 
                now_playing_resources = None;
            };
            // Update the last snapshot time, used to determine the player position when rendering
            last_snapshot_time = Instant::now();
        }

        // Clear the canvas for drawing
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.set_blend_mode(BlendMode::None);
        canvas.clear();

        // Deactivate all of the buttons
        for button in buttons.values_mut() {
            button.active = false;
        }

        // If the player data has been received, run this code
        if let Some(u_np_resources) = &now_playing_resources {
            let info_tex = u_np_resources.track_resources.info_texture();
            let info_qry = info_tex.query();
            let art_tex = u_np_resources.track_resources.artwork_texture();
            
            if u_np_resources.player_info.state() == PlayerState::Playing {
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
            //TODO: find a way to avoid nesting this if statement inside the last
            if window_input_focus && window_rect.contains_point(Point::new(mouse_state.x(), mouse_state.y())) || window_interaction_in_progress {
                // Darken the cover art
                canvas.set_blend_mode(BlendMode::Mod);
                canvas.set_draw_color(Color::RGB( 120, 120, 120));
                canvas.fill_rect(Rect::new(0, 0, ARTWORK_SIZE, ARTWORK_SIZE)).unwrap();

                // Draw a progress bar
                canvas.set_blend_mode(BlendMode::Add);
                canvas.set_draw_color(Color::RGB(100, 100, 100));

                let percent_elapsed = (u_np_resources.player_info.pos() 
                    + if u_np_resources.player_info.state() == PlayerState::Playing { last_snapshot_time.elapsed().as_secs_f64() } else { 0. })
                    / u_np_resources.track_info.length();

                canvas.draw_line(
                    Point::new(0, ARTWORK_SIZE as i32 - 1), 
                    Point::new(
                        (ARTWORK_SIZE as f64 * percent_elapsed) as i32, 
                        ARTWORK_SIZE as i32 - 1
                    )
                ).unwrap();

                // Update button visibility based on new data
                buttons.get_mut("next_track").unwrap().active = true;
                buttons.get_mut("back_track").unwrap().active = true;

                buttons.get_mut("minimize").unwrap().active = true;
                buttons.get_mut("close").unwrap().active = true;   

                buttons.get_mut("heart_empty").unwrap().active = !u_np_resources.track_info.loved();
                buttons.get_mut("heart_filled").unwrap().active = u_np_resources.track_info.loved();

                buttons.get_mut("play").unwrap().active = u_np_resources.player_info.state() != PlayerState::Playing;
                buttons.get_mut("pause").unwrap().active = u_np_resources.player_info.state() == PlayerState::Playing;
            }
        } else {
            // TODO: This should ideally be unnecessary
            buttons.get_mut("minimize").unwrap().active = true;
            buttons.get_mut("close").unwrap().active = true;
        }

        // Draw each button and add its rect to the 'sub' vec if it's active, then deactivate every button
        sub.clear();
        for button in buttons.values() { 
            button.render(&mut canvas, mouse_state).unwrap(); 
            if button.active {
                sub.push(raw_heap_rect(button.collision_rect.x, button.collision_rect.y, button.collision_rect.w, button.collision_rect.h));
            }
        }
        hit_test_data.sub_len = sub.len() as c_int;
        hit_test_data.sub = sub.as_ptr();
       
    
        //Draw a border
        canvas.set_blend_mode(BlendMode::Mod);
        canvas.set_draw_color(Color::RGB(200, 200, 200));
        canvas.draw_rect(Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT)).unwrap();
        canvas.set_blend_mode(BlendMode::Add);
        canvas.set_draw_color(Color::RGB(30, 30, 30));
        canvas.draw_rect(Rect::new(1, 1, WINDOW_WIDTH-2, WINDOW_HEIGHT-2)).unwrap();

        // Debug the length of the album resources array
        if let Some(album_resources) = album_resources.as_deref() { dbg!(album_resources.len()); }
        
        //Present the canvas
        canvas.present();
        thread::sleep(Duration::from_nanos(1_000_000_000u64 / 30));
    }
}
