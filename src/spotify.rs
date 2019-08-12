use rfmod::Sys;
use termion::event::Key;
use tui::terminal::Frame;
use tui::layout::Rect;
use tui::backend::Backend;
use rspotify::spotify::client::Spotify;
use rspotify::spotify::util::get_token;
use rspotify::spotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use super::player::Player;
use super::{ Config, MediaPlayer };

pub struct SpotifyPlayer {
    config: Config
}

impl SpotifyPlayer {
    pub fn new(config: Config) -> SpotifyPlayer {
        let mut oauth = SpotifyOAuth::default()
            .scope("user-read-recently-played")
            .build();
        match get_token(&mut oauth) {
            Some(token_info) => {
                let client_credential = SpotifyClientCredentials::default()
                    .token_info(token_info)
                    .build();
                let spotify = Spotify::default()
                    .client_credentials_manager(client_credential)
                    .build();
                let playlists = spotify.current_user_playlists(10, None);
                println!("{:?}", playlists);
            }
            None => println!("auth failed"),
        };

        SpotifyPlayer {
            config
        }
    }
}

impl Player for SpotifyPlayer {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect, media_player: &mut MediaPlayer) {
        
    }

    fn input(&mut self, key: Key, fmod: &Sys, media_player: &mut MediaPlayer) {
        match key {
            _ => {}
        }
    }

    fn tick(&mut self, fmod: &Sys, media_player: &mut MediaPlayer) {
        
    }
}