use data_encoding::BASE64URL_NOPAD;
use directories::ProjectDirs;
use sdl2::{
    image::LoadTexture,
    render::{Texture, TextureCreator},
};
use serde::Deserialize;
use std::path::Path;

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
    artwork: Texture<'a>,
}

impl<'a> AlbumResources<'a> {
    pub fn artwork(&self) -> &Texture<'a> {
        &self.artwork
    }
    pub fn title(&self) -> &str {
        &self.base_resources.album
    }
    pub fn album_artist(&self) -> &str {
        &self.base_resources.album_artist
    }
}

/// A subset of AlbumResources that doesn't contain a texture, allowing for it to be passed between threads.
pub struct BaseAlbumResources {
    // tracks: Vec<String>,
    album: String,
    album_artist: String,
    artwork_file_path: String,
}

impl BaseAlbumResources {
    pub fn build(
        response: ADOsascriptResponse,
        artwork_cache_dir: &Path,
        artwork_size: u32,
    ) -> Self {
        let filename = format!(
            "{}.png",
            BASE64URL_NOPAD
                .encode(format!("{}{}", &response.album_artist, &response.album).as_bytes())
        );

        let path = artwork_cache_dir
            .join(filename)
            .to_str()
            .unwrap()
            .to_owned();

        if let Some(artwork_data) = response.artwork_data {
            crate::engine::raw_to_cached_image(&artwork_data, (artwork_size, artwork_size), &path)
                .unwrap();
        }

        Self {
            // tracks: response.tracks,
            album: response.album,
            album_artist: response.album_artist,
            artwork_file_path: path,
        }
    }

    pub fn construct_artwork<T>(self, texture_creator: &TextureCreator<T>) -> AlbumResources {
        sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "best"); // linear filtering

        let artwork = texture_creator
            .load_texture(&self.artwork_file_path)
            .unwrap(); // Should never panic

        sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "nearest"); // point filtering

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
        std::fs::create_dir_all(artwork_cache_dir).expect("Unable to create cache directory.");

        println!("Getting a list of cached albums...");
        let album_cache: Vec<String> = std::fs::read_dir(artwork_cache_dir)
            .unwrap()
            .map(|file| {
                let mut filename = file.unwrap().file_name().into_string().unwrap();
                filename.truncate(filename.len() - 4);
                String::from_utf8(BASE64URL_NOPAD.decode(filename.as_bytes()).unwrap()).unwrap()
            })
            .collect();

        println!("Getting raw data from Apple Music...");
        let mut album_data = osascript_requests::get_album_data(album_cache);

        println!("Removing duplicate albums...");
        album_data.sort_by(|a, b| {
            if a.album_artist > b.album_artist
                || (a.album_artist == b.album_artist && a.album > b.album)
            {
                std::cmp::Ordering::Greater
            } else if a.album == b.album {
                std::cmp::Ordering::Equal
            } else {
                std::cmp::Ordering::Less
            }
        });
        album_data.dedup_by(|a, b| a.album == b.album && a.album_artist == b.album_artist);

        println!("Building resources...");
        let base_album_resources: Vec<BaseAlbumResources> = album_data
            .into_iter()
            .map(|response| BaseAlbumResources::build(response, artwork_cache_dir, artwork_size))
            .collect();

        println!("Done!");
        base_album_resources
    }
}
