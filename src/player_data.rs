use sdl2::image::LoadTexture;
use sdl2::pixels::Color;
use sdl2::render::Texture;
use sdl2::render::TextureCreator;

use sdl2::video::WindowContext;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PDOsascriptResponse {
    pub track_info: TrackInfo,
    pub player_info: PlayerInfo,
    pub track_artwork_data: String,
}

#[derive(Clone, Copy, Deserialize, PartialEq)]
pub enum PlayerState {
    Stopped,
    Playing,
    Paused,
    FastForwarding,
    Rewinding,
}

/// Information about a track, including name, artist, album, loved, and length.
#[derive(Deserialize, PartialEq)]
pub struct TrackInfo {
    name: String,
    artist: String,
    album: String,
    loved: bool,
    length: f64,
}
impl TrackInfo {
    pub fn name(&self) -> &str {
        return &self.name;
    }
    pub fn artist(&self) -> &str {
        return &self.artist;
    }
    pub fn album(&self) -> &str {
        return &self.album;
    }
    pub fn loved(&self) -> bool {
        return self.loved;
    }
    pub fn length(&self) -> f64 {
        self.length
    }
}

/// Information about the player itself, including the player position (time elapsed in current song) and state.
#[derive(Deserialize)]
pub struct PlayerInfo {
    pos: f64,
    state: PlayerState,
}
impl PlayerInfo {
    pub fn pos(&self) -> f64 {
        return self.pos;
    }
    pub fn state(&self) -> PlayerState {
        return self.state;
    }
}

/// Computation-heavy texture resources for a track, including a description and artwork texture. Should only be created when a track
/// is first loaded.
#[allow(dead_code)]
pub struct TrackResources<'a> {
    info_texture: Texture<'a>,
    artwork_texture: Texture<'a>,
}

#[allow(dead_code)]
impl<'a> TrackResources<'a> {
    // TODO: find a better way to determine foreground and background color for the texture than passing them as parameters to this function
    pub fn new<T: 'a>(
        response: &PDOsascriptResponse,
        texture_creator: &'a TextureCreator<T>,
    ) -> Result<TrackResources<'a>, Box<dyn std::error::Error>> {
        //Create a texture from the album info
        let track_info = &response.track_info;
        let info_texture = crate::engine::text_to_texture(
            &format!(
                "{} - {} - {}",
                track_info.artist, track_info.name, track_info.album
            ),
            &texture_creator,
            Color::RGB(255, 255, 255),
            Color::RGB(0, 0, 0),
        );

        let artwork_texture = crate::engine::raw_to_texture(&response.track_artwork_data, texture_creator)?;

        Ok(TrackResources {
            info_texture,
            artwork_texture,
        })
    }

    pub fn info_texture(&self) -> &Texture {
        return &self.info_texture;
    }
    pub fn artwork_texture(&self) -> &Texture {
        return &self.artwork_texture;
    }
}

/// A collection of resources representing all of the data received in and parsed from the original OsascriptResponse.
pub struct NowPlayingResourceCollection<'a> {
    pub player_info: PlayerInfo,
    pub track_info: TrackInfo,
    pub track_resources: TrackResources<'a>,
}

impl<'a> NowPlayingResourceCollection<'a> {
    pub fn build(
        response: PDOsascriptResponse,
        texture_creator: &'a TextureCreator<WindowContext>,
    ) -> NowPlayingResourceCollection<'a> {
        let track_resources = TrackResources::new(&response, texture_creator).unwrap();
        NowPlayingResourceCollection {
            player_info: response.player_info,
            track_info: response.track_info,
            track_resources,
        }
    }
    pub fn update(
        &mut self,
        response: PDOsascriptResponse,
        texture_creator: &'a TextureCreator<WindowContext>,
    ) {
        // Determine whether track resources need to be recreated by comparing old player data with new player data
        if self.track_info != response.track_info {
            self.track_resources = TrackResources::new(&response, texture_creator).unwrap();
            self.track_info = response.track_info;
        }
        self.player_info = response.player_info;
    }
}
