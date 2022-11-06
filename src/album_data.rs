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

pub struct AlbumResources<'a> {
    // tracks: Vec<String>,
    album: String,
    album_artist: String,
    artwork: Texture<'a>
}

impl<'a> AlbumResources<'a> {
    pub fn build<T>(response: ADOsascriptResponse, texture_creator: &'a TextureCreator<T>, artwork_cache_dir: &Path) -> Self {
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
        
        let artwork = {
            if let Some(artwork_data) = response.artwork_data {
                crate::engine::raw_to_cached_image(
                    &artwork_data,
                    (100, 100), 
                    &path,
                ).unwrap();
            }
            texture_creator.load_texture(path).unwrap()  // Should never panic
        }; 

        Self {
            // tracks: response.tracks,
            album: response.album,
            album_artist: response.album_artist,
            artwork,
        }
    }

    pub fn get_all_from_music<T>(texture_creator: &'a TextureCreator<T>) -> Vec<Self> {
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

        println!("Building textures...");
        let album_resources: Vec<AlbumResources> = album_data
            .into_iter()
            .map(|response| 
                AlbumResources::build(
                    response, 
                    &texture_creator, 
                    artwork_cache_dir, 
                ))
            .collect();
            
        println!("Done!");
        album_resources
    }
}
