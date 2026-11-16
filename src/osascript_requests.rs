use mpd::search::Window;
use mpd::{Client, Query, Term};
use osascript;
use serde::Serialize;

use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use crate::album_data::ADOsascriptResponse;
use crate::player_data::{PDOsascriptResponse, PlayerInfo, PlayerState, TrackInfo};

type PlayerDataSender = Sender<Option<PDOsascriptResponse>>;

trait MySong {
    fn get_album(&self) -> Option<&String>;
    fn get_album_artist(&self) -> Option<&String>;
}

impl MySong for mpd::Song {
    fn get_album(&self) -> Option<&String> {
        self.tags
            .iter()
            .find_map(|(k, v)| if k == "Album" {Some(v)} else {None})
    }
    fn get_album_artist(&self) -> Option<&String> {
        self.tags
            .iter()
            .find_map(|(k, v)| if k == "AlbumArtist" {Some(v)} else {None})
    }
}

/// Returns information on the state of the music player
fn get_player_data(client: &mut Client, mut last_player_data: Option<PDOsascriptResponse>) -> Option<PDOsascriptResponse> {
    // TODO: make it so that this is toggled based on OS
    // const PLAYER_DATA_SCRIPT: &'static str = include_str!("osascript_requests/get_player_data.jxa");
    // let script = osascript::JavaScript::new(PLAYER_DATA_SCRIPT);
    // script.execute().ok()

    println!("Getting track info");
    let song = client.currentsong().ok()??; // HACK bad bad bad wth
    let track_info = TrackInfo {
        name: song.title.clone().unwrap_or("No title".to_string()),
        artist: song.artist.clone().unwrap_or("No artist".to_string()),
        album: song.get_album().map(String::to_owned).unwrap_or("No album".to_string()),
        loved: false, // HACK
        length: song.duration.unwrap().as_secs_f64(), // if theres not a duration, oops
    };
    println!("Getting status");
    let status = client.status().ok()?; // HACK again, bad bad bad
    let state: PlayerState = match status.state {
        mpd::State::Stop => PlayerState::Stopped,
        mpd::State::Play => PlayerState::Playing,
        mpd::State::Pause => PlayerState::Paused
    };
    let player_info = PlayerInfo {
        pos: status.elapsed.unwrap().as_secs_f64(), // oops 
        dj_active: false, // TODO
        state
    };
    let track_artwork_data = if let Some(lpd) = last_player_data.take() {
        if lpd.track_info == track_info {
            println!("Using cached artwork");
            lpd.track_artwork_data
        } else {
            println!("Getting artwork");
            // horrendous code. but it just has to work (positive affirmations)
            client.albumart(&song).ok()?
        }
    } else {
        println!("Getting artwork");
        client.albumart(&song).ok()?
    };
    println!("Done");
    return Some(PDOsascriptResponse { track_info, player_info, track_artwork_data })
}

/// Sends information about the music player's state to the main thread
fn send_player_data(data: Option<PDOsascriptResponse>, tx: PlayerDataSender) {
    tx.send(data)
        .expect("Couldn't send player data through the channel");
}

/// Creates a new thread to gather information on the current track and send it to the main thread once complete.
pub fn send_player_data_async(client: Arc<Mutex<Client>>, tx: PlayerDataSender) {
    thread::spawn(move || {
        let mut lock = client.lock().unwrap();
        let data = get_player_data(&mut lock, None);
        send_player_data(data, tx);
    });
}

/// Periodically runs a JXA script to gather information on the current track and send it to the main thread.
pub fn send_player_data_loop(client: Arc<Mutex<Client>>, tx: PlayerDataSender) {
    thread::spawn(move || {
        let mut time_remaining = 3.;
        let mut cur_player_data = None;
        loop {
            println!("Getting player data");
            let data = {
                let mut lock = client.lock().unwrap();
                get_player_data(&mut lock, cur_player_data)
            };
            if let Some(response) = data.as_ref() {
                time_remaining = response.track_info.length() - response.player_info.pos();
            }
            send_player_data(data.clone(), tx.clone());
            cur_player_data = data;
            // If the track is almost over, don't sleep the full duration so the info can be updated immediately after it ends
            thread::sleep(Duration::from_secs_f64(time_remaining.min(3.0).max(0.2)));
        }
    });
}

/// Clears all played tracks from the micromusic DJ playlist. Does nothing if there is an error.
fn clear_played_tracks() {
    // const CLEAR_PLAYED_SCRIPT: &'static str = include_str!("osascript_requests/clear_played_tracks.jxa");
    // let script = osascript::JavaScript::new(CLEAR_PLAYED_SCRIPT);
    // let _: () = script.execute().unwrap_or_else(|e| println!("Unable to clear played tracks: {e}"));
}

/// Periodically runs a JXA script to clear played tracks from the micromusic DJ playlist
pub fn clear_played_tracks_loop() {
    thread::spawn(move || {
        loop {
            clear_played_tracks();
            // If the track is almost over, don't sleep the full duration so the info can be updated immediately after it ends
            thread::sleep(Duration::from_secs_f64(10.0));
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
            // JXACommand::Love => "Application('Music').currentTrack.loved = true",
            // JXACommand::Unlove => "Application('Music').currentTrack.loved = false",
            JXACommand::Love => "try { Application('Music').currentTrack.loved = true } catch { Application('Music').currentTrack.favorited = true }", 
            JXACommand::Unlove => "try { Application('Music').currentTrack.loved = false } catch { Application('Music').currentTrack.favorited = false }", 
        }
    }
}

/// Can run different JXA commands depending on the "command" parameter
/// * `tx` - An MPSC sender to optionally update the player data on the main thread after the command has completed. Set it to None to disable this behavior.
pub fn run_command<T>(client: Arc<Mutex<Client>>, command: JXACommand, tx: T, player_state: PlayerState)
where
    T: Into<Option<PlayerDataSender>>,
{
    let tx = tx.into();
    // let script = osascript::JavaScript::new(command.as_str());
    thread::spawn(move || {
        // let _: () = script.execute().unwrap();
        println!("getting lock");
        let mut lock = client.lock().unwrap();
        println!("got lock");
        match command {
            JXACommand::PlayPause => {
                match player_state {
                    PlayerState::Paused => lock.toggle_pause(), // not play(), tested to not work
                    PlayerState::Playing => lock.toggle_pause(),
                    PlayerState::Stopped => lock.play(),
                    _ => todo!()
                }.unwrap();
            }
            JXACommand::NextTrack => { lock.next().unwrap(); }
            JXACommand::PreviousTrack => { lock.prev().unwrap(); }
            JXACommand::BackTrack => { lock.prev().unwrap(); } // HACK
            JXACommand::Love => { todo!() }
            JXACommand::Unlove => { todo!() }
        }
        if let Some(tx) = tx {
            send_player_data_async(client.clone(), tx);
        }
    });
}

#[derive(Serialize)]
struct GetAlbumDataParams {
    cached_albums: Vec<String>,
}

/// Gets data for the album selection screen. Should only be run one time at the start of the program.
pub fn get_album_data(client: &mut Client, album_cache: Vec<String>) -> Vec<ADOsascriptResponse> {
    // const ALBUM_DATA_SCRIPT: &'static str = include_str!("osascript_requests/get_album_data.jxa");
    // let script = osascript::JavaScript::new(ALBUM_DATA_SCRIPT);
    // let result: Vec<ADOsascriptResponse> = script
    //     .execute_with_params(GetAlbumDataParams {
    //         cached_albums: album_cache,
    //     })
    //     .unwrap();
    // result
    let mut albums = Vec::new();
    let mut album_cache_set: HashSet<String> = HashSet::from_iter(album_cache.into_iter());
    let songs = client.listall().unwrap();
    for song in songs {
        let mut q = Query::new();
        q.and(Term::File, song.file.clone());
        let song = client.find(&q, None).unwrap().into_iter().next().unwrap();

        // println!("{}", song.file);
        // XXX unnecessary now
        if let (Some(album), Some(album_artist)) = (song.get_album(), song.get_album_artist().or(song.artist.as_ref())) {
            let id = format!("{album_artist}{album}");
            if album_cache_set.contains(&id) {
                println!("{} -> none", song.file);
                albums.push(ADOsascriptResponse {
                    album: album.to_owned(),
                    album_artist: album_artist.to_owned(),
                    artwork_data: None, // stupid unreadable code. None means it's alr cached, but we want to know that it still exists, so we have to add it to the vec without the art
                })
            } else if let Ok(artwork_data) = client.albumart(&song) {
                println!("{} -> artwork", song.file);
                albums.push(ADOsascriptResponse {
                    album: album.to_owned(),
                    album_artist: album_artist.to_owned(),
                    artwork_data: Some(artwork_data)
                });
                album_cache_set.insert(id);
            }
            // if there is no artwork available, do nothing
        }
    }

    return albums

// for (var i = 0; i < tracks.length; i++) {
//     const track = tracks[i];
//     const album = track.album();
//     const albumLower = album.toLowerCase();

//     if (albumLower != prevAlbumLower) {
//         prevAlbumLower = albumLower;
        
//         const albumArtist = track.albumArtist()

//         let artworkData = null;
//         if (!cachedAlbums.has(albumArtist + album)) {
//             if (tracks[i].artworks.length === 0) {
//                 // just ignore tracks without artwork
//                 continue;
//             }
//             artworkData = tracks[i].artworks[0].rawData()
//         }
//         albums.push({
//             // tracks: [trackName],
//             artwork_data: artworkData,
//             album: album,
//             album_artist: albumArtist,
//         });
//     }
// }
}

#[derive(Serialize)]
struct PlayAlbumParams {
    album: String,
    album_artist: String,
}

pub fn queue_album(client: Arc<Mutex<Client>>, tx: PlayerDataSender, album: String, album_artist: String){
    // const ALBUM_PLAY_SCRIPT: &'static str = include_str!("osascript_requests/queue_album.jxa");
    // let script = osascript::JavaScript::new(ALBUM_PLAY_SCRIPT);

    let queue_album_client = client.clone();
    thread::spawn(move || {
        // let _: () = script
        //     .execute_with_params(PlayAlbumParams {
        //         album,
        //         album_artist,
        //     })
        //     .unwrap();
        let mut lock = queue_album_client.lock().unwrap();
        let mut query = Query::new();
        query.and(Term::Tag(Cow::from("Album")), album).and(Term::Tag(Cow::from("AlbumArtist")), album_artist);
        lock.findadd(&query).unwrap();
        lock.play().unwrap();
    });
    send_player_data_async(client.clone(), tx);
}

pub fn make_dj_playlist() {
    // const MAKE_DJ_PLAYLIST_SCRIPT: &'static str =
    //     include_str!("osascript_requests/make_dj_playlist.jxa");
    // let script = osascript::JavaScript::new(MAKE_DJ_PLAYLIST_SCRIPT);
    // thread::spawn(move || {
    //     let _: () = script.execute().unwrap();
    // });
}

