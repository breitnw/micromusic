use osascript;
use serde::Serialize;

use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use crate::album_data::ADOsascriptResponse;
use crate::player_data::PDOsascriptResponse;

type PlayerDataSender = Sender<Option<PDOsascriptResponse>>;

/// Returns information on the state of the music player
fn get_player_data() -> Option<PDOsascriptResponse> {
    const PLAYER_DATA_SCRIPT: &'static str = include_str!("osascript_requests/get_player_data.jxa");
    let script = osascript::JavaScript::new(PLAYER_DATA_SCRIPT);
    script.execute().ok()
}

/// Sends information about the music player's state to the main thread
fn send_player_data(data: Option<PDOsascriptResponse>, tx: PlayerDataSender) {
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
    const ALBUM_DATA_SCRIPT: &'static str = include_str!("osascript_requests/get_album_data.jxa");
    let script = osascript::JavaScript::new(ALBUM_DATA_SCRIPT);
    let result: Vec<ADOsascriptResponse> = script
        .execute_with_params(GetAlbumDataParams {
            cached_albums: album_cache,
        })
        .unwrap();
    result
}

#[derive(Serialize)]
struct PlayAlbumParams {
    album: String,
    album_artist: String,
}

pub fn queue_album(album: String, album_artist: String) {
    const ALBUM_PLAY_SCRIPT: &'static str = include_str!("osascript_requests/queue_album.jxa");
    let script = osascript::JavaScript::new(ALBUM_PLAY_SCRIPT);

    thread::spawn(move || {
        let _: () = script
            .execute_with_params(PlayAlbumParams {
                album,
                album_artist,
            })
            .unwrap();
    });
}

pub fn make_dj_playlist() {
    const MAKE_DJ_PLAYLIST_SCRIPT: &'static str =
        include_str!("osascript_requests/make_dj_playlist.jxa");
    let script = osascript::JavaScript::new(MAKE_DJ_PLAYLIST_SCRIPT);
    thread::spawn(move || {
        let _: () = script.execute().unwrap();
    });
}

pub fn remove_dj_playlist() {
    const REMOVE_DJ_PLAYLIST_SCRIPT: &'static str =
        include_str!("osascript_requests/remove_dj_playlist.jxa");
    let script = osascript::JavaScript::new(REMOVE_DJ_PLAYLIST_SCRIPT);
    thread::spawn(move || {
        let _: () = script.execute().unwrap();
    });
}

