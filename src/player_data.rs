use sdl2::image::LoadTexture;
use sdl2::render::Texture;
use sdl2::render::TextureCreator;

use serde::Deserialize;

use hex::FromHex;
use sdl2::rwops::RWops;
use sdl2::image::ImageRWops;


#[derive(Deserialize)]
pub struct PlayerData {
    pub track_name: String,
    pub track_artist: String,
    pub track_album: String,
    pub track_loved: bool,
    pub track_length: f64,
    pub track_artwork_data: Option<String>,

    pub player_pos: f64,
    pub player_state: String, //"stopped"/‌"playing"/‌"paused"/‌"fast forwarding"/‌"rewinding"
}

impl PlayerData {
    /// Creates a clone of the player data, keeping artwork_data as "None" to save on resources.
    pub fn clone_without_artwork(&self) -> PlayerData {
        PlayerData {
            track_artwork_data: None,

            track_name: self.track_name.to_owned(),
            track_artist: self.track_artist.to_owned(),
            track_album: self.track_album.to_owned(),
            track_loved: self.track_loved,
            track_length: self.track_length,
            player_pos: self.player_pos,
            player_state: self.player_state.to_owned(),
        }
    }
}

#[allow(dead_code)]
pub struct TrackData<'a> {
    name: String,
    artist: String,
    album: String,
    loved: bool,
    info_texture: Texture<'a>,
    artwork_texture: Texture<'a>,
}

// TODO: not necessary to create a new TrackData every time data is received, only when song changes
#[allow(dead_code)]
impl<'a> TrackData<'a> {
    pub fn new<T: 'a>(data: &PlayerData, texture_creator: &'a TextureCreator<T>) -> Result<TrackData<'a>, Box<dyn std::error::Error>> {
        //Create a texture from the album info
        let info_texture = crate::engine::text_to_texture(
            &format!("{} - {} - {}", data.track_artist, data.track_name, data.track_album),
            &texture_creator,
        );

        // Load the raw bytes for the album artwork
        let raw_artwork_data: &str = &data.track_artwork_data.as_ref().ok_or("Artwork data must not be None")?;
        let bytes = Vec::from_hex(&raw_artwork_data[8..raw_artwork_data.len() - 2])?;

        // Use linear filtering when creating the texture 
        sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "linear");

        // Use the texture creator to create the texture
        let artwork_texture = texture_creator.load_texture_bytes(&bytes).unwrap();
        
        // Reset filtering to nearest
        sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "nearest");

        Ok(TrackData {
            name: data.track_name.to_owned(),
            artist: data.track_artist.to_owned(),
            album: data.track_album.to_owned(),
            loved: data.track_loved,
            info_texture,
            artwork_texture
        })
    }

    pub fn info_texture(&self) -> &Texture { return &self.info_texture }
    pub fn artwork_texture(&self) -> &Texture { return &self.artwork_texture }

    pub fn name(&self) -> &String { return &self.name }
    pub fn artist(&self) -> &String { return &self.artist }
    pub fn album(&self) -> &String { return &self.album }
    pub fn loved(&self) -> bool { return self.loved }
}
