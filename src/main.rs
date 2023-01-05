
// FRONT BURNER
// TODO: temporarily add albums to a new array for reshuffle animation
// TODO: center "not playing" text (current hack doesn't actually center it) + reset info scroll pos when it's not zero
// TODO: add gradient at the top of album selection view to make buttons more visible
// TODO: add a box in the bottom right where users can drag albums to queue them, maybe show a number on the box
// TODO: re-implement variable framerate if necessary


// CRASHES
// TODO: Probably panic the whole program when the secondary thread panics
// TODO: Fix crash when clicking heart button while nothing's playing

// FIXES
// TODO: Antialiasing issues moving back and forth between displays (try regenerating texture or setting hint when changing screens)
// TODO: Some albums getting rendered multiple times

// BACK BURNER
// TODO: Filter albums by genre
// TODO: Draw overlay buttons on "not playing" screen
// TODO: Only re-render info text every frame, not album art
// TODO: Remember window position on close
// TODO: Add a way for user to manually clear caches
// TODO: Load data directly from cache, then update it when loaded from apple music


use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};
use std::sync::mpsc;
use std::{thread};

use queues::{Queue, IsQueue};
use rust_embed::RustEmbed;

use sdl2;
use sdl2::image::LoadTexture;
use sdl2::libc::c_int;
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::event::{Event, WindowEvent};
use sdl2::rect::{Rect, Point};
use sdl2::render::{ TextureCreator, Texture, BlendMode };

use sdl2::sys::{SDL_SetWindowHitTest, SDL_Window, SDL_Point, SDL_Rect, SDL_HitTestResult};
use std::ffi::c_void;

use rand::thread_rng;
use rand::seq::SliceRandom;

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


#[derive(PartialEq)]
enum View {
    Miniplayer,
    AlbumSelect,
}

// PRIMARY THREAD: Renders a SDL2 interface for users to interact with the application
fn main() {
    sdl2::hint::set("SDL_VIDEO_ALLOW_SCREENSAVER", "1");
    
    // Set up a MPSC channel to send player data between threads
    let (player_tx, player_rx) = mpsc::channel();

    // Spawn a secondary thread to periodically gather information on the current track and send it to the main thread
    osascript_requests::send_player_data_loop(player_tx.clone());
    osascript_requests::make_dj_playlist();

    // Initialize SDL
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    // Sizes for the window and canvas
    const ARTWORK_SIZE: u32 = 210;
    const INFO_AREA_HEIGHT: u32 = 40;
    const INFO_PADDING: u32 = 10;

    const WINDOW_WIDTH: u32 = ARTWORK_SIZE;
    const WINDOW_HEIGHT: u32 = ARTWORK_SIZE + INFO_AREA_HEIGHT;
    let window_rect = Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT);
    let artwork_rect = Rect::new(0, 0, ARTWORK_SIZE, ARTWORK_SIZE);

    const THUMBNAIL_SIZE: u32 = ARTWORK_SIZE / 3;
    const I_THUMBNAIL_SIZE: i32 = ARTWORK_SIZE as i32 / 3;
    const THUMBNAIL_SCALE_AMT_HOVER: u32 = 10; // added
    const THUMBNAIL_SCALE_AMT_DRAG: u32 = 20; // subtracted
    const THUMBNAIL_SIZE_DRAG: u32 = THUMBNAIL_SIZE - THUMBNAIL_SCALE_AMT_DRAG;

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

    const ICON_COLOR_MOD_DEFAULT: u8 = 150;
    const ICON_COLOR_MOD_HOVER: u8 = 255;
    let icon_textures_default = load_icons(ICON_COLOR_MOD_DEFAULT, BlendMode::Add, &texture_creator);
    let icon_textures_hover = load_icons(ICON_COLOR_MOD_HOVER, BlendMode::Add, &texture_creator);

    // Create a map of all of the buttons on the screen
    let button_data = {
        const A_SIZE: i32 = ARTWORK_SIZE as i32;
        [
            ("album_view", (5, 5)),
            ("miniplayer_view", (5, 5)), 
            ("heart_empty", (19, 5)),
            ("heart_filled", (19, 5)),
            ("reshuffle", (19, 5)),
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
    
    // Add the queue box button, which requires a different texture for default and hover states
    buttons.insert("queue", Button::new(ARTWORK_SIZE as i32 - 36, ARTWORK_SIZE as i32 - 36, 
        &icon_textures_default["queue_closed.png"], 
        &icon_textures_hover["queue_open.png"], 
        &icon_textures_hover["queue_open.png"], 
    ));
    let mut queue_button_squash_frame = 0;
    
    // Spawn a thread to get all album resources from Apple Music
    let (album_tx, album_rx) = mpsc::channel();
    thread::spawn(move || {
        let base_album_resources = BaseAlbumResources::get_all_from_music(ARTWORK_SIZE * 2 / 3);
        album_tx.send(base_album_resources).unwrap();
    });
    

    // Data for album select screen (Rc necessary because Queue requires Clone and references will go out of scope)
    enum ItemState {
        Default,
        Dragged,
        Freeze { timer: u32},
        Anim {
            duration: u32,
            current: u32, 
            x_curve: Box<dyn Fn(f32) -> f32>,
            y_curve: Box<dyn Fn(f32) -> f32>, 
            scale_curve: Box<dyn Fn(f32) -> f32>,
        },
    }
    struct AlbumViewItem<'a> { 
        pub album: Rc<AlbumResources<'a>>, 
        pub x_vel: f32,
        pub x_pos: f32,
        pub state: ItemState
    }
    impl AlbumViewItem<'_> {
        fn get_target_pos(item_col_i: usize, drag_placeholder_x: Option<usize>) -> usize {
            let mut item_col_i = item_col_i;
            let drag_placeholder_x = drag_placeholder_x.filter(|x| item_col_i >= *x);
            if drag_placeholder_x.is_some() {
                item_col_i += 1;
            }
            item_col_i as usize * ARTWORK_SIZE as usize / 3
        }
        fn update(&mut self, target_x: f32) {
            let dist_from_target = self.x_pos - target_x;

            match &mut self.state {
                // Move the thumbnail according to its position and velocity
                ItemState::Default => {
                    if dist_from_target > 2.0 {
                        self.x_vel = (dist_from_target / 100.0).abs().sqrt() * -15.0;
                    } else if dist_from_target < -2.0 {
                        self.x_vel = (dist_from_target / 100.0).abs().sqrt() * 30.0;
                    } else {
                        self.x_vel = 0.0;
                        self.x_pos = target_x;
                    }
                    self.x_pos += self.x_vel;
                }
                ItemState::Freeze { timer } => {
                    // Freeze any thumbnails with the freeze timers greater than 0. Primarily used to stagger movements
                    if *timer > 0 {
                        *timer -= 1;
                    } else {
                        self.state = ItemState::Default;
                    }
                }
                ItemState::Anim { duration, current, .. } => {
                    if current < duration {
                        *current += 1;
                    } else {
                        self.state = ItemState::Default;
                    }
                }
                _ => ()
            }
        }
        fn get_anim_pos_and_scale(&self) -> Option<([f32; 2], f32)> {
            if let ItemState::Anim { 
                duration, 
                current, 
                x_curve, 
                y_curve, 
                scale_curve,
                ..
            } = &self.state {
                let progress = *current as f32 / *duration as f32;
                return Some(([x_curve(progress), y_curve(progress)], scale_curve(progress)));
            } else {
                return None;
            }
        }
    }
    let mut album_view_queue: Queue<Rc<AlbumResources>> = Queue::new();
    let mut album_view_rows: [Vec<AlbumViewItem>; 3] = [vec![], vec![], vec![]];
    let mut dragged_item: Option<AlbumViewItem> = None;
    let mut dragged_item_pos: [f32; 2] = [0.0, 0.0];
    let mut drag_placeholder_loc: Option<[usize; 2]> = None;
    let mut queueing_albums: Vec<AlbumViewItem> = Vec::new();


    // State variables for the rendering loop
    let mut now_playing_resources: NowPlayingResourceCollection = NowPlayingResourceCollection::build(None, &texture_creator);
    let mut last_snapshot_time = Instant::now();
    let mut info_scroll_pos: i32 = 0;
    const INFO_SPACING: i32 = 50;

    let mut current_view = View::Miniplayer;

    // A boolean that specifies whether the user is currently dragging the window. Set when a window drag event occurs,
    // reset when the mouse button is released
    let mut window_interaction_in_progress = false;

    // This code will run every frame
    'running: loop {

        // Mouse state
        let mouse_state = engine::mouse::MouseState::get_relative_state(&event_pump, canvas.window());
        let window_input_focus = &canvas.window().window_flags() & 512 == 512; // input focus: 512, mouse focus: 1024
        
        // Iterate through the input events
        for event in event_pump.poll_iter() { match event {
            Event::Quit { .. } => {
                break 'running;
            },
            Event::MouseButtonDown { x, y, mouse_btn: MouseButton::Left, ..} => {
                if Button::get_hovered_from_hash(&buttons, x, y).is_some() {
                    continue;
                }
                if current_view == View::AlbumSelect && artwork_rect.contains_point(mouse_state.pos()) {
                    let hovered_album_loc = mouse_state.pos() / I_THUMBNAIL_SIZE;
                    let target_row = album_view_rows.get_mut(hovered_album_loc.y as usize).unwrap();
                    
                    if !target_row.is_empty() {
                        let mut target_item = target_row.remove(hovered_album_loc.x as usize);
                        target_item.state = ItemState::Dragged;
                        dragged_item_pos = [
                            target_item.x_pos + (THUMBNAIL_SCALE_AMT_DRAG / 2) as f32,
                            (I_THUMBNAIL_SIZE * hovered_album_loc.y) as f32 + (THUMBNAIL_SCALE_AMT_DRAG / 2) as f32
                        ];
                        dragged_item = Some(target_item);

                        target_row.push(AlbumViewItem {
                            album: album_view_queue.remove().unwrap(),
                            state: ItemState::Default,
                            x_pos: ARTWORK_SIZE as f32,
                            x_vel: 0.0,
                        });
                    }
                }
            }
            Event::MouseButtonUp { x, y, mouse_btn: MouseButton::Left, ..} => {
                // Filter loc so that it's none if out of bounds
                let loc = drag_placeholder_loc
                    .filter(|loc| (loc[1]) < 3 && (loc[0]) < 3);

                if let Some(mut u_dragged_item) = dragged_item.take() {
                    if buttons["queue"].is_hovering(mouse_state.x(), mouse_state.y()) {
                        // Constants
                        const START_SCALE: f32 = THUMBNAIL_SIZE_DRAG as f32 / THUMBNAIL_SIZE as f32;
                        const START_OFFSET: f32 = THUMBNAIL_SIZE_DRAG as f32 / 2.0;
                        
                        // The starting x and y positions of the thumbnail
                        let (x, y) = (dragged_item_pos[0] + START_OFFSET, dragged_item_pos[1] + START_OFFSET);

                        // Quadratic data for vertical animation curve
                        const QUEUE_BUTTON_X: f32 = ARTWORK_SIZE as f32 - 20.0;
                        const QUEUE_BUTTON_Y: f32 = ARTWORK_SIZE as f32 - 20.0;
                        const ANIM_ARC_HEIGHT: f32 = 150.0;
                        
                        let a = QUEUE_BUTTON_Y - y;
                        let b = -2.0 * (ANIM_ARC_HEIGHT - y);
                        let c = ANIM_ARC_HEIGHT - y;
                        let vertex_t = (-b + (b.powi(2) - 4.0 * a * c).sqrt()) / (2.0 * a);
                        let scale_y = (y - ANIM_ARC_HEIGHT) / vertex_t.powi(2);

                        // Set the dragged item's state to animate
                        u_dragged_item.state = ItemState::Anim { 
                            duration: 15, 
                            current: 0, 
                            x_curve: Box::new(move |t: f32| x + t * (QUEUE_BUTTON_X - x)), 
                            y_curve: Box::new(move |t: f32| scale_y * (t - vertex_t).powi(2) + ANIM_ARC_HEIGHT), 
                            scale_curve: Box::new(|t: f32| (1.0 - t) * START_SCALE), 
                        };
                        queueing_albums.push(u_dragged_item);

                        // Freeze the next item in the third row until the animation finishes
                        album_view_rows[2][2].state = ItemState::Freeze { timer: 25 };
                        continue;
                    }
                    u_dragged_item.state = ItemState::Default;
                    if let Some(loc) = loc {
                        u_dragged_item.x_pos = AlbumViewItem::get_target_pos(loc[0], None) as f32;
                        album_view_rows[loc[1]].insert(loc[0], u_dragged_item);
                    } else {
                        album_view_queue.add(u_dragged_item.album).unwrap();
                    }
                    dragged_item = None;
                    continue;
                }
                
                match Button::get_hovered_from_hash(&buttons, x, y) {
                    Some(button_name) => match button_name {
                        "heart_empty" => osascript_requests::run_command(JXACommand::Love, player_tx.clone()),
                        "heart_filled" => osascript_requests::run_command(JXACommand::Unlove, player_tx.clone()),
                        "album_view" => current_view = View::AlbumSelect,
                        "miniplayer_view" => current_view = View::Miniplayer,
                        "play" | "pause" => osascript_requests::run_command(JXACommand::PlayPause, player_tx.clone()),
                        "back_track" => osascript_requests::run_command(JXACommand::BackTrack, player_tx.clone()),
                        "next_track" => osascript_requests::run_command(JXACommand::NextTrack, player_tx.clone()),
                        "minimize" => {
                            // TODO: temporary border enabling hack no longer necessary in Ventura, remove?
                            canvas.window_mut().set_bordered(true);
                            canvas.window_mut().minimize(); 
                            canvas.window_mut().set_bordered(false);
                        }
                        "reshuffle" => {
                            for row in album_view_rows.iter_mut() {
                                row.drain(..).for_each(|item| { album_view_queue.add(item.album).unwrap(); } );
                            }
                            for i in 0..9 {
                                if let Ok(a) = album_view_queue.remove() {
                                    album_view_rows[i % 3].push(AlbumViewItem {
                                        album: a, 
                                        x_pos: AlbumViewItem::get_target_pos(i / 3, None) as f32 + ARTWORK_SIZE as f32, 
                                        x_vel: 0.0,
                                        state: ItemState::Freeze { timer: i as u32 % 3 * 3 + i as u32 / 3 }
                                    })
                                } else {
                                    break;
                                }
                            }
                        }
                        "close" => {
                            // osascript_requests::remove_dj_playlist();
                            break 'running
                        }
                        _ => {}
                    }
                    None => ()
                }
            }
            Event::Window { win_event, .. } => {
                match win_event {
                    WindowEvent::Moved {..} => {
                        // Update the canvas scale in case the user drags the window to a different monitor
                        engine::update_canvas_scale(&mut canvas, WINDOW_WIDTH, WINDOW_HEIGHT);
                    },
                    _ => {}
                }
            }
            _ => {}
        }}

        // Reset drag_in_progress if the mouse button was just lifted
        if mouse_state.is_mouse_button_pressed(sdl2::mouse::MouseButton::Left) {
            if window_rect.contains_point(mouse_state.pos()) && window_input_focus {
                window_interaction_in_progress = true;
            }
        } else {
            window_interaction_in_progress = false;
        }

        
        // If the now playing channel has new data in it, update the player and track data on this thread
        if let Ok(response) = player_rx.try_recv() {
            now_playing_resources.update(response, &texture_creator);
            // Update the last snapshot time, used to determine the player position when rendering
            last_snapshot_time = Instant::now();
        }

        // If the base album resources are done loading, create and save their artwork textures
        if let Ok(response) = album_rx.try_recv() {
            let mut album_resources: Vec<AlbumResources> = response.into_iter()
                .map(|r| r.construct_artwork(&texture_creator))
                .collect();
            
            album_resources.shuffle(&mut thread_rng());
            album_resources.into_iter().for_each(|item| { 
                album_view_queue.add(Rc::new(item)).unwrap(); 
            });

            for i in 0..9 {
                if let Ok(a) = album_view_queue.remove() {
                    album_view_rows[i % 3].push(AlbumViewItem {
                        album: a, 
                        x_pos: AlbumViewItem::get_target_pos(i / 3, None) as f32, 
                        x_vel: 0.0,
                        state: ItemState::Default,
                    })
                } else {
                    break;
                }
            }
        }


        // Clear the canvas for drawing
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.set_blend_mode(BlendMode::None);
        canvas.clear();

        // Deactivate all of the buttons
        for button in buttons.values_mut() {
            button.active = false;
        }


        // Per-frame code for the album select screen
        if current_view == View::AlbumSelect {

            // The 3x3 grid coordinates of the album that's currently being hovered over
            // Casts done individually to prevent negative positions rounding toward 0
            let hovered_album_loc: [usize; 2] = [
                (mouse_state.x() as usize) / THUMBNAIL_SIZE as usize, 
                (mouse_state.y() as usize) / THUMBNAIL_SIZE as usize
            ];
            drag_placeholder_loc = dragged_item.as_ref().map(|_| hovered_album_loc);

            // Update the positions of all of the thumbnails and draw them
            for (row_y, row) in album_view_rows.iter_mut().enumerate() {
                for (item_x, item) in row.iter_mut().enumerate() {
                    let drag_placeholder_x = drag_placeholder_loc.filter(|loc| loc[1] == row_y).map(|loc| loc[0]);
                    let target_x = AlbumViewItem::get_target_pos(item_x, drag_placeholder_x) as f32;
                    item.update(target_x);

                    let thumbnail_rect = Rect::new(
                        item.x_pos as i32,
                        I_THUMBNAIL_SIZE * row_y as i32, 
                        THUMBNAIL_SIZE, 
                        THUMBNAIL_SIZE
                    );
                    canvas.copy(item.album.artwork(), None, thumbnail_rect).unwrap();
                }
            }

            // Draw the item that is currently being dragged, if there is one
            if let Some(u_dragged_item) = dragged_item.as_ref() {
                const DRAG_SPEED_MULTIPLIER: f32 = 0.6;

                let target_pos = mouse_state.pos() - Point::new(THUMBNAIL_SIZE_DRAG as i32 / 2, THUMBNAIL_SIZE_DRAG as i32 / 2);
                let direction = [target_pos.x() as f32 - dragged_item_pos[0], target_pos.y() as f32 - dragged_item_pos[1]];
                let velocity = [direction[0] * DRAG_SPEED_MULTIPLIER, direction[1] * DRAG_SPEED_MULTIPLIER];

                dragged_item_pos = [dragged_item_pos[0] + velocity[0], dragged_item_pos[1] + velocity[1]];
                let dragged_item_rect = Rect::new(
                    dragged_item_pos[0] as i32,
                    dragged_item_pos[1] as i32,
                    THUMBNAIL_SIZE_DRAG, 
                    THUMBNAIL_SIZE_DRAG,
                );
                canvas.copy(u_dragged_item.album.artwork(), None, dragged_item_rect).unwrap();
            
            } else if window_input_focus && window_rect.contains_point(mouse_state.pos()) {
                // Enlarge the album artwork that the user is hovering over
                let target_row = album_view_rows.get_mut(hovered_album_loc[1]);

                if let Some(item) = target_row.and_then(|r| r.get(hovered_album_loc[0])) {
                    if item.x_vel.abs() < 0.1 && matches!(item.state, ItemState::Default) {
                        let thumbnail_rect = Rect::new(
                            item.x_pos as i32 - THUMBNAIL_SCALE_AMT_HOVER as i32 / 2, 
                            I_THUMBNAIL_SIZE * hovered_album_loc[1] as i32 - THUMBNAIL_SCALE_AMT_HOVER as i32 / 2, 
                            THUMBNAIL_SIZE + THUMBNAIL_SCALE_AMT_HOVER, 
                            THUMBNAIL_SIZE + THUMBNAIL_SCALE_AMT_HOVER,
                        );
                        canvas.copy(item.album.artwork(), None, thumbnail_rect).unwrap();
                    }
                }
            }

            buttons.get_mut("queue").unwrap().texture_default = &icon_textures_default["queue_closed.png"];
            buttons.get_mut("queue").unwrap().texture_hover = &icon_textures_hover["queue_open.png"];
            buttons.get_mut("queue").unwrap().texture_pressed = &icon_textures_hover["queue_open.png"];

            for i in 0..queueing_albums.len() {
                buttons.get_mut("queue").unwrap().texture_default = &icon_textures_default["queue_open.png"];
                queueing_albums[i].update(0.0);
                if let Some((pos, scale)) = queueing_albums[i].get_anim_pos_and_scale() {
                    let scale = (THUMBNAIL_SIZE as f32 * scale) as u32;
                    let thumbnail_rect = Rect::new(
                        pos[0] as i32 - scale as i32 / 2,
                        pos[1] as i32 - scale as i32 / 2, 
                        scale, 
                        scale,
                    );
                    canvas.copy(queueing_albums[i].album.artwork(), None, thumbnail_rect).unwrap();
                } else {
                    queue_button_squash_frame = 2;
                    let album = queueing_albums.remove(i).album;
                    osascript_requests::queue_album(album.title().to_string(), album.album_artist().to_string());
                    album_view_queue.add(album).unwrap();
                }
            }

            buttons.get_mut("minimize").unwrap().active = true;
            buttons.get_mut("close").unwrap().active = true;  
            buttons.get_mut("miniplayer_view").unwrap().active = true;
            buttons.get_mut("reshuffle").unwrap().active = true;

            buttons.get_mut("queue").unwrap().active = true;

            // Squash animation for the queue button after an album is queued
            match queue_button_squash_frame {
                2 => {
                    buttons.get_mut("queue").unwrap().texture_default = &icon_textures_default["queue_squashed_2.png"];
                    buttons.get_mut("queue").unwrap().texture_hover = &icon_textures_hover["queue_squashed_2.png"];
                    buttons.get_mut("queue").unwrap().texture_pressed = &icon_textures_hover["queue_squashed_2.png"];
                    queue_button_squash_frame -= 1;
                }
                1 => {
                    buttons.get_mut("queue").unwrap().texture_default = &icon_textures_default["queue_squashed_1.png"];
                    buttons.get_mut("queue").unwrap().texture_hover = &icon_textures_hover["queue_squashed_1.png"];
                    buttons.get_mut("queue").unwrap().texture_pressed = &icon_textures_hover["queue_squashed_1.png"];
                    queue_button_squash_frame -= 1;
                }
                _ => {}
            }

            canvas.set_draw_color(Color::BLACK);
            canvas.fill_rect(Rect::new(0, ARTWORK_SIZE as i32, ARTWORK_SIZE, INFO_AREA_HEIGHT)).unwrap();
        }


        // Draw info depending on the received player data
        let info_tex = now_playing_resources.track_resources.info_texture();
        let info_qry = info_tex.query();
        let art_tex = now_playing_resources.track_resources.artwork_texture();
        
        if now_playing_resources.player_info.state() == PlayerState::Playing {
            info_scroll_pos -= 1; 
            info_scroll_pos %= info_qry.width as i32 + INFO_SPACING;
        }

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

        if current_view == View::Miniplayer {
            // Draw the album art
            canvas.copy(art_tex, None, Rect::new(0,0, 
                ARTWORK_SIZE, 
                ARTWORK_SIZE
            )).unwrap();

            //Draw an overlay if the user is hovering over the window
            //TODO: find a way to avoid nesting this if statement inside the last
            if window_input_focus && window_rect.contains_point(mouse_state.pos()) || window_interaction_in_progress {
                // Darken the cover art
                canvas.set_blend_mode(BlendMode::Mod);
                canvas.set_draw_color(Color::RGB( 120, 120, 120));
                canvas.fill_rect(Rect::new(0, 0, ARTWORK_SIZE, ARTWORK_SIZE)).unwrap();

                // Draw a progress bar
                canvas.set_blend_mode(BlendMode::Add);
                canvas.set_draw_color(Color::RGB(100, 100, 100));

                let percent_elapsed = (now_playing_resources.player_info.pos() 
                    + if now_playing_resources.player_info.state() == PlayerState::Playing { last_snapshot_time.elapsed().as_secs_f64() } else { 0. })
                    / now_playing_resources.track_info.length();

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

                buttons.get_mut("heart_empty").unwrap().active = !now_playing_resources.track_info.loved();
                buttons.get_mut("heart_filled").unwrap().active = now_playing_resources.track_info.loved();

                buttons.get_mut("album_view").unwrap().active = true;

                buttons.get_mut("play").unwrap().active = now_playing_resources.player_info.state() != PlayerState::Playing;
                buttons.get_mut("pause").unwrap().active = now_playing_resources.player_info.state() == PlayerState::Playing;
            }
        }

        // Draw each button and add its rect to the 'sub' vec if it's active, then deactivate every button
        sub.clear();
        for button in buttons.values() { 
            button.render(&mut canvas, mouse_state).unwrap(); 
            if button.active {
                sub.push(raw_heap_rect(button.collision_rect.x, button.collision_rect.y, button.collision_rect.w, button.collision_rect.h));
            }
        }
        if current_view == View::AlbumSelect {
            if dragged_item.is_some() {
                sub.push(raw_heap_rect(0, 0, WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32));
            } else {
                sub.push(raw_heap_rect(0, 0, ARTWORK_SIZE as i32, ARTWORK_SIZE as i32));
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
        
        //Present the canvas
        canvas.present();
        thread::sleep(Duration::from_nanos(1_000_000_000u64 / 30));
    }
}

