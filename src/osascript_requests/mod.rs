
use osascript;

use std::time::Duration;
use std::sync::mpsc::Sender;
use std::thread;

use crate::player_data::PlayerData;

type PlayerDataSender = Sender<Option<PlayerData>>;

/// Gathers information on the current track and sends it to the main thread once complete.
fn send_player_data(tx: PlayerDataSender) -> Option<PlayerData> {
    // let err_test: RawTrackData = script.execute().unwrap();
    const PLAYER_INFO_SCRIPT: &str = include_str!("get_player_data.jxa");
    let script = osascript::JavaScript::new(PLAYER_INFO_SCRIPT);

    match script.execute() {
        Ok::<PlayerData, _>(player_data) => {
            let cloned_pl_data = player_data.clone_without_artwork();
            tx.send(Some(player_data)).expect("Couldn't send player data through the channel");
            return Some(cloned_pl_data)
        }
        Err(e) => {
            tx.send(None).expect("Couldn't send player data through the channel");
            println!("Error receiving data from Apple Music: {e}");
            return None
        }
    }
}


/// Creates a new thread to gather information on the current track and send it to the main thread once complete.
pub fn send_player_data_async(tx: PlayerDataSender) {
    thread::spawn(move || { send_player_data(tx) });
}


/// Periodically runs a JXA script to gather information on the current track and send it to the main thread.
pub fn send_player_data_loop(tx: PlayerDataSender) {
    thread::spawn(move || {
        let mut time_remaining = 3.;
        loop {
            if let Some(player_data) = send_player_data(tx.clone()) {
                time_remaining = player_data.track_length - player_data.player_pos;
            }
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
            JXACommand::Unlove => "Application('Music').currentTrack.loved = false"
        }
    }
}


/// Can run different JXA commands depending on the "command" parameter
/// * `tx` - An MPSC sender to optionally update the player data on the main thread after the command has completed. Set it to None to disable this behavior.
pub fn run_command<T>(command: JXACommand, tx: T)
where T: Into<Option<PlayerDataSender>> {
    let tx = tx.into();
    let script = osascript::JavaScript::new(command.as_str());
    thread::spawn( move || {
        let _: () = script.execute().unwrap();
        if let Some(tx) = tx {
            send_player_data_async(tx);
        }
    });
}
