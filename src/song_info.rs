use sdl2::render::Texture;
use sdl2::render::TextureCreator;
use sdl2::render::TextureQuery;

pub struct RawSongData {
    name: String,
    artist: String,
    album: String,
}

impl RawSongData {
    pub fn new(name: String, artist: String, album: String) -> RawSongData {
        RawSongData { name, artist, album}
    }
    pub fn to_string(&self) -> String {
        format!("{} - {} - {}", self.artist, self.name, self.album)
    }
}

pub struct SongData<'a> {
    data: RawSongData,
    texture: Texture<'a>,
    texture_query: TextureQuery,
}

impl<'a> SongData<'a> {
    pub fn new<T: 'a>(data: RawSongData, texture_creator: &'a TextureCreator<T>) -> SongData<'a> {
        let texture = super::text_to_texture(
            &data.to_string(), 
            &texture_creator,
        );
        SongData {
            data,
            texture_query: texture.query(),
            texture,
        }
    }
    pub fn data(&self) -> &RawSongData { return &self.data }
    pub fn texture(&self) -> &Texture { return &self.texture }
    pub fn texture_query(&self) -> &TextureQuery { return &self.texture_query }
}