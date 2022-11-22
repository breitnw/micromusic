use osascript;
use serde::Serialize;

use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use crate::player_data::PDOsascriptResponse;
use crate::album_data::ADOsascriptResponse;

type PlayerDataSender = Sender<Option<PDOsascriptResponse>>;

/// Returns information on the state of the music player
fn get_player_data() -> Option<PDOsascriptResponse> {
    const PLAYER_DATA_SCRIPT: &'static str = include_str!("get_player_data.jxa");
    let script = osascript::JavaScript::new(PLAYER_DATA_SCRIPT);
    script.execute().ok()
}

/// Sends information about the music player's state to the main thread
fn send_player_data(data: Option<PDOsascriptResponse>, tx: PlayerDataSender) {
    if data.is_none() {
        println!("Error receiving player data from Apple Music");
    }
    tx.send(data)
        .expect("Couldn't send player data through the channel");
}

/// Creates a new thread to gather information on the current track and send it to the main thread once complete.
pub fn send_player_data_async(tx: PlayerDataSender) {
    thread::spawn(move || {
        let data = get_player_data();
        send_player_data(data, tx);
    });
}

/// Periodically runs a JXA script to gather information on the current track and send it to the main thread.
pub fn send_player_data_loop(tx: PlayerDataSender) {
    thread::spawn(move || {
        let mut time_remaining = 3.;
        loop {
            let data = get_player_data();
            if let Some(response) = data.as_ref() {
                time_remaining = response.track_info.length() - response.player_info.pos();
            }
            send_player_data(data, tx.clone());
            // If the track is almost over, don't sleep the full duration so the info can be updated immediately after it ends
            thread::sleep(Duration::from_secs_f64(time_remaining.min(3.0).max(0.2)));
        }
    });
}

#[allow(dead_code)]
pub enum JXACommand {
    PlayPause,
    NextTrack,
    PreviousTrack,
    BackTrack,
    Love,
    Unlove,
}

impl JXACommand {
    fn as_str(&self) -> &'static str {
        match self {
            JXACommand::PlayPause => "Application('Music').playpause()",
            JXACommand::NextTrack => "Application('Music').nextTrack()",
            JXACommand::PreviousTrack => "Application('Music').previousTrack()",
            JXACommand::BackTrack => "Application('Music').backTrack()",
            JXACommand::Love => "Application('Music').currentTrack.loved = true",
            JXACommand::Unlove => "Application('Music').currentTrack.loved = false",
        }
    }
}

/// Can run different JXA commands depending on the "command" parameter
/// * `tx` - An MPSC sender to optionally update the player data on the main thread after the command has completed. Set it to None to disable this behavior.
pub fn run_command<T>(command: JXACommand, tx: T)
where
    T: Into<Option<PlayerDataSender>>,
{
    let tx = tx.into();
    let script = osascript::JavaScript::new(command.as_str());
    thread::spawn(move || {
        let _: () = script.execute().unwrap();
        if let Some(tx) = tx {
            send_player_data_async(tx);
        }
    });
}

#[derive(Serialize)]
struct GetAlbumDataParams {
    cached_albums: Vec<String>,
}

/// Gets data for the album selection screen. Should only be run one time at the start of the program. 
pub fn get_album_data(album_cache: Vec<String>) -> Vec<ADOsascriptResponse> {
    const ALBUM_DATA_SCRIPT: &'static str = include_str!("get_album_data.jxa");
    let script = osascript::JavaScript::new(ALBUM_DATA_SCRIPT);
    let result: Vec<ADOsascriptResponse> = script.execute_with_params(GetAlbumDataParams { cached_albums: album_cache }).unwrap();
    result
}