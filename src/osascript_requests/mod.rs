
use osascript;

use std::time::Duration;
use std::sync::mpsc;
use std::thread;

use crate::player_data::PlayerData;


/// Gathers information on the current song and sends it to the main thread once complete.
fn send_player_data(tx: mpsc::Sender<Option<PlayerData>>) -> Option<PlayerData> {
    // let err_test: RawSongData = script.execute().unwrap();
    const PLAYER_INFO_SCRIPT: &str = include_str!("get_player_data.jxa");
    let script = osascript::JavaScript::new(PLAYER_INFO_SCRIPT);

    if let Ok::<PlayerData, _>(player_data) = script.execute() {
        let cloned_pl_data = player_data.clone_without_artwork();
        tx.send(Some(player_data)).expect("Couldn't send player data through the channel");
        Some(cloned_pl_data)
    } 
    else {
        tx.send(None).expect("Couldn't send player data through the channel");
        println!("Error receiving data from Apple Music.");
        None
    }
}


/// Creates a new thread to gather information on the current song and send it to the main thread once complete.
pub fn send_player_data_async(tx: mpsc::Sender<Option<PlayerData>>) {
    thread::spawn(move || { send_player_data(tx) });
}


/// Periodically runs a JXA script to gather information on the current song and send it to the main thread.
pub fn send_player_data_loop(tx: mpsc::Sender<Option<PlayerData>>) {
    thread::spawn(move || {
        let mut time_remaining = -1.;
        loop {
            if let Some(player_data) = send_player_data(tx.clone()) {
                time_remaining = player_data.song_length - player_data.player_pos;
            }
            // If the song is almost over, don't sleep the full duration so the info can be updated immediately after it ends
            thread::sleep(Duration::from_secs_f64(time_remaining.min(3.)));
        }
    });
}


/// Runs the "playpause() JXA command
/// * `tx` - An MPSC sender to optionally update the player data on the main thread after the command has completed. Set it to None to disable this behavior.
pub fn playpause<T>(tx: T)
where T: Into<Option<mpsc::Sender<Option<PlayerData>>>> {
    let tx = tx.into();
    let script = osascript::JavaScript::new("Application('Music').playpause()");
    thread::spawn( move || {
        let _: () = script.execute().unwrap();
        if let Some(tx) = tx {
            send_player_data_async(tx);
        }
    });
}
