use sdl2::render::Texture;
use sdl2::render::TextureCreator;

use serde::Deserialize;

use hex::FromHex;
use sdl2::rwops::RWops;
use sdl2::image::ImageRWops;


#[derive(Deserialize)]
pub struct PlayerData {
    pub song_name: String,
    pub song_artist: String,
    pub song_album: String,
    pub song_length: f64,
    pub artwork_data: Option<String>,

    pub player_pos: f64,
    pub player_state: String, //"stopped"/‌"playing"/‌"paused"/‌"fast forwarding"/‌"rewinding"
}

impl PlayerData {
    /// Creates a clone of the player data, keeping artwork_data as "None" to save on resources.
    pub fn clone_without_artwork(&self) -> PlayerData {
        PlayerData {
            artwork_data: None,
            song_name: self.song_name.to_owned(),
            song_artist: self.song_artist.to_owned(),
            song_album: self.song_album.to_owned(),
            song_length: self.song_length,
            player_pos: self.player_pos,
            player_state: self.player_state.to_owned(),
        }
    }
}


pub struct SongData<'a> {
    name: String,
    artist: String,
    album: String,
    info_texture: Texture<'a>,
    artwork_texture: Texture<'a>,
}

impl<'a> SongData<'a> {
    pub fn new<T: 'a>(data: &PlayerData, texture_creator: &'a TextureCreator<T>) -> Result<SongData<'a>, Box<dyn std::error::Error>> {
        //Create a texture from the album info
        let info_texture = crate::engine::text_to_texture(
            &format!("{} - {} - {}", data.song_artist, data.song_name, data.song_album),
            &texture_creator,
        );

        // Load the album artwork texture from the raw bytes and store it in artwork_texture
        let raw_artwork_data: &str = &data.artwork_data.as_ref().ok_or("Artwork data must not be None")?;
        let bytes = Vec::from_hex(&raw_artwork_data[8..raw_artwork_data.len() - 2])?;
        let rw = RWops::from_bytes(&bytes)?;
        let artwork_texture = texture_creator.create_texture_from_surface(
            rw.load().expect("Couldn't load image from bytes")
        ).unwrap();

        Ok(SongData {
            name: data.song_name.to_owned(),
            artist: data.song_artist.to_owned(),
            album: data.song_album.to_owned(),
            info_texture,
            artwork_texture
        })
    }

    pub fn info_texture(&self) -> &Texture { return &self.info_texture }
    pub fn artwork_texture(&self) -> &Texture { return &self.artwork_texture }

    pub fn name(&self) -> &String {return &self.name }
    pub fn artist(&self) -> &String {return &self.artist }
    pub fn album(&self) -> &String {return &self.album }
}