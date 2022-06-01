use sdl2::render::Texture;
use sdl2::render::TextureCreator;
use sdl2::render::TextureQuery;

use serde::Deserialize;

use hex::FromHex;
use sdl2::rwops::RWops;
use sdl2::image::ImageRWops;


#[derive(Deserialize, Clone)]
pub struct RawPlayerData {
    pub song_name: String,
    pub song_artist: String,
    pub song_album: String,
    pub song_length: f64,
    pub artwork_data: String,

    pub player_pos: f64,
    pub player_state: String, //"stopped"/‌"playing"/‌"paused"/‌"fast forwarding"/‌"rewinding"
}


pub struct SongData<'a> {
    name: String,
    artist: String,
    album: String,
    info_texture: Texture<'a>,
    info_texture_query: TextureQuery,
    artwork_texture: Texture<'a>,
    artwork_texture_query: TextureQuery,
}

impl<'a> SongData<'a> {
    pub fn new<T: 'a>(data: RawPlayerData, texture_creator: &'a TextureCreator<T>) -> SongData<'a> {
        //Create a texture from the album info
        let info_texture = super::text_to_texture(
            &format!("{} - {} - {}", data.song_artist, data.song_name, data.song_album),
            &texture_creator,
        );

        // Load the album artwork texture from the raw bytes and store it in artwork_texture
        // TODO: Lots of unwraps here, ideally propagate errors instead
        // let hex_data = std::fs::read_to_string("src/test_data.txt").unwrap();
        let hex_data = &data.artwork_data[8..data.artwork_data.len() - 2];

        let bytes = Vec::from_hex(hex_data).expect("Couldn't convert pixel data to bytes");
        let rw = RWops::from_bytes(&bytes).unwrap();
        let artwork_texture = texture_creator.create_texture_from_surface(
            rw.load().expect("Couldn't load image from bytes")
        ).unwrap();

        SongData {
            name: data.song_name,
            artist: data.song_artist,
            album: data.song_album,
            info_texture_query: info_texture.query(),
            info_texture,
            artwork_texture_query: artwork_texture.query(),
            artwork_texture
        }
    }

    pub fn info_texture(&self) -> &Texture { return &self.info_texture }
    pub fn info_texture_query(&self) -> &TextureQuery { return &self.info_texture_query }

    pub fn artwork_texture(&self) -> &Texture { return &self.artwork_texture }
    pub fn artwork_texture_query(&self) -> &TextureQuery { return &self.artwork_texture_query }

    pub fn name(&self) -> &String {return &self.name }
    pub fn artist(&self) -> &String {return &self.artist }
    pub fn album(&self) -> &String {return &self.album }
}