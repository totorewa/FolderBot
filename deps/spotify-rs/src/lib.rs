#![warn(missing_docs)]
//! The Spotify crate.
//!
//! This crate contains methods to retrieve information from
//! and manipulate the local Spotify client instance.

// Extern crates
extern crate winapi;
extern crate kernel32;
extern crate reqwest;
extern crate time;
extern crate json;

// Modules
#[cfg(windows)]
mod windows_process;
mod connector;
pub mod status;

// Imports
#[cfg(windows)]
use windows_process::WindowsProcess;
use connector::{SpotifyConnector, InternalSpotifyError};
use status::{SpotifyStatus};

/// The `Result` type used in this crate.
type Result<T> = std::result::Result<T, SpotifyError>;

/// The `SpotifyError` enum.
#[derive(Debug)]
pub enum SpotifyError {
    /// An internal error.
    InternalError(InternalSpotifyError),
    /// Indicates that the Spotify Client is not running.
    ClientNotRunning,
    /// Indicates that the SpotifyWebHelper process it not running.
    WebHelperNotRunning,
}

/// The Spotify API.
pub struct Spotify {
    /// The Spotify connector.
    connector: SpotifyConnector,
}

/// Fetches the current status from Spotify.
async fn get_status(connector: &SpotifyConnector) -> Result<SpotifyStatus> {
    match connector.fetch_status_json().await {
        Ok(result) => Ok(SpotifyStatus::from(result)),
        Err(error) => Err(SpotifyError::InternalError(error)),
    }
}

/// Implements `Spotify`.
impl Spotify {
    /// Connects to the local Spotify client.
    #[cfg(windows)]
    pub async fn connect() -> Result<Spotify> {
        // TODO:
        // At some point, the connector should automatically
        // open Spotify in the case  that Spotify is closed.
        // That would also be a much better cross-platform solution,
        // because it would work on Linux and macOS and make
        // the dependency on winapi and kernel32-sys unnecessary.
        if !Spotify::spotify_webhelper_alive() {
            return Err(SpotifyError::WebHelperNotRunning);
        }
        Spotify::new_unchecked().await
    }
    /// Connects to the local Spotify client.
    #[cfg(not(windows))]
    pub async fn connect() -> Result<Spotify> {
        Spotify::new_unchecked().await
    }
    /// Constructs a new `self::Result<Spotify>`.
    async fn new_unchecked() -> Result<Spotify> {
        match SpotifyConnector::connect_new().await {
            Ok(result) => Ok(Spotify { connector: result }),
            Err(error) => Err(SpotifyError::InternalError(error)),
        }
    }
    /// Fetches the current status from the client.
    pub async fn status(&self) -> Result<SpotifyStatus> {
        get_status(&self.connector).await
    }
    /// Plays a track.
    pub async fn play(&self, track: String) -> bool {
        // Try to fix broken track URIs
        // In: https://open.spotify.com/track/1pGZIV8olkbRMjyHWoEXyt
        // In: open.spotify.com/track/1pGZIV8olkbRMjyHWoEXyt
        // In: track/1pGZIV8olkbRMjyHWoEXyt
        // In: track:1pGZIV8olkbRMjyHWoEXyt
        // Out: spotify:track:1pGZIV8olkbRMjyHWoEXyt
        let track: String = {
            let track = track
                .replace("https://", "http://") // https -> http
                .trim_start_matches("http://") // get rid of protocol
                .trim_start_matches("open.spotify.com") // get rid of domain name
                .replace("/", ":") // turn all / into :
                .trim_start_matches(":") // get rid of : at the beginning
                .to_owned();
            if track.starts_with("spotify:") {
                track
            } else {
                format!("spotify:{}", track) // prepend proper protocol
            }
        };
        // Play the track
        self.connector.request_play(track).await
    }
    /// Pauses the currently playing track.
    /// Has no effect if the track is already paused.
    pub async fn pause(&self) -> bool {
        self.connector.request_pause(true).await
    }
    /// Resumes the currently paused track.
    /// Has no effect if the track is already playing.
    pub async fn resume(&self) -> bool {
        self.connector.request_pause(false).await
    }
    /// Tests whether the SpotifyWebHelper process is running.
    #[cfg(windows)]
    fn spotify_webhelper_alive() -> bool {
        let process = "SpotifyWebHelper.exe";
        WindowsProcess::find_by_name(process).is_some()
    }
}
