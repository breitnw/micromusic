use sdl2::{render::{Texture, TextureCreator}, image::LoadTexture};
use serde::Deserialize;
use std::path::Path;
use directories::ProjectDirs;
use data_encoding::BASE64URL_NOPAD;

use crate::osascript_requests;


#[derive(Deserialize, Debug)]
pub struct ADOsascriptResponse {
    // pub tracks: Vec<String>,
    pub album: String,
    pub album_artist: String,
    pub artwork_data: Option<String>,
}

/// A set of resources for displaying information about albums.
pub struct AlbumResources<'a> {
    base_resources: BaseAlbumResources,
    artwork: Texture<'a>
}

impl<'a> AlbumResources<'a> {
    pub fn base_resources(&self) -> &BaseAlbumResources { &self.base_resources }
    pub fn artwork(&self) -> &Texture<'a> { &self.artwork }
}

/// A subset of AlbumResources that doesn't contain a texture, allowing for it to be passed between threads.
pub struct BaseAlbumResources {
    // tracks: Vec<String>,
    album: String,
    album_artist: String,
    artwork_file_path: String,
}

impl BaseAlbumResources {
    pub fn build(response: ADOsascriptResponse, artwork_cache_dir: &Path, artwork_size: u32) -> Self {
        let filename = format!(
            "{}.png", 
            // sea::hash64(format!("{}{}", &response.album_artist, &response.album).as_bytes())
            BASE64URL_NOPAD.encode(format!("{}{}", &response.album_artist, &response.album).as_bytes())
        );

        let path = artwork_cache_dir
            .join(filename)
            .to_str()
            .unwrap()
            .to_owned();
        
        if let Some(artwork_data) = response.artwork_data {
            crate::engine::raw_to_cached_image(
                &artwork_data,
                (artwork_size, artwork_size), 
                &path,
            ).unwrap();
        }

        Self {
            // tracks: response.tracks,
            album: response.album,
            album_artist: response.album_artist,
            artwork_file_path: path,
        }
    }

    pub fn construct_artwork<'a, T>(self, texture_creator: &'a TextureCreator<T>) -> AlbumResources {
        let artwork = texture_creator.load_texture(&self.artwork_file_path).unwrap();  // Should never panic
        AlbumResources {
            base_resources: self,
            artwork,
        }
    }

    pub fn get_all_from_music(artwork_size: u32) -> Vec<Self> {
        // Create the cache directory at ~/Library/Caches/com.breitnw.micromusic/artwork/
        let project_dirs = ProjectDirs::from("com", "breitnw", "micromusic")
            .expect("Unable to get path to cache directory.");
        let cache_dir: &Path = &project_dirs.cache_dir();
        let artwork_cache_dir: &Path = &cache_dir.join("artwork");
        std::fs::create_dir_all(artwork_cache_dir)
            .expect("Unable to create cache directory.");
        
        println!("Getting a list of cached albums...");
        let album_cache: Vec<String> = std::fs::read_dir(artwork_cache_dir).unwrap().map(|file| {
            let mut filename = file.unwrap().file_name().into_string().unwrap();
            filename.truncate(filename.len() - 4);
            String::from_utf8(BASE64URL_NOPAD.decode(filename.as_bytes()).unwrap()).unwrap()
        }).collect();
            
        println!("Getting raw data from Apple Music...");
        let album_data = osascript_requests::get_album_data(album_cache);

        println!("Building resources...");
        let base_album_resources: Vec<BaseAlbumResources> = album_data
            .into_iter()
            .map(|response| 
                BaseAlbumResources::build(
                    response,
                    artwork_cache_dir, 
                    artwork_size,
                ))
            .collect();
            
        println!("Done!");
        base_album_resources
    }
}
