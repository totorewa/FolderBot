use rspotify::{prelude::*, scopes, AuthCodeSpotify, Config, Credentials, OAuth};

pub struct SpotifyChecker {
    pub spotify: AuthCodeSpotify,
}

impl SpotifyChecker {
    pub async fn new() -> SpotifyChecker {
        // spotify
        let rspotify_config = Config {
            token_cached: true,
            token_refreshing: true,
            ..Default::default()
        };
        let creds = Credentials::from_env().unwrap();
        let scopes = scopes!("user-read-currently-playing");
        let oauth = OAuth::from_env(scopes).unwrap();
        let spotify = AuthCodeSpotify::with_config(creds, oauth, rspotify_config);
        let url = spotify.get_authorize_url(false).unwrap();
        spotify.prompt_for_token(&url).await.unwrap();

        // spotify.add_item_to_queue("https://open.spotify.com/track/3ZEno9fORwMA1HPecdLi0R", None);

        SpotifyChecker {
            spotify
        }
    }
}
