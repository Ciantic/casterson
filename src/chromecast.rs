/// Chromecast default media reciever
///
/// # Example
///
/// ```rust
/// let rec = get_default_media_receiver("192.168.8.106")
/// rec.cast("http://commondatastorage.googleapis.com/gtv-videos-bucket/big_buck_bunny_1080p.mp4").unwrap();
///
/// rec.pause().unwrap();
/// rec.play().unwrap();
/// // reciever.stop();
/// ```
extern crate rust_cast;

use derive_more::From;
use rust_cast::channels::connection::ConnectionResponse;
use rust_cast::channels::heartbeat::HeartbeatResponse;
use rust_cast::channels::media::MediaResponse;
use rust_cast::channels::media::{IdleReason, Media, PlayerState, StatusEntry, StreamType};
use rust_cast::channels::receiver::CastDeviceApp;
use rust_cast::{CastDevice, ChannelMessage};
use serde::Serializer;
use std::net::IpAddr;
use url::Url;

use serde::Serialize;
use std::str::FromStr;

const DEFAULT_DESTINATION_ID: &str = "receiver-0";
const DEFAULT_PORT: u16 = 8009;

pub fn get_default_media_receiver(
    ip: &IpAddr,
    port: Option<u16>,
    dest_id: Option<String>,
) -> MediaReceiver {
    MediaReceiver {
        ip: *ip,
        port: port.unwrap_or(DEFAULT_PORT),
        dest_id: dest_id.unwrap_or(DEFAULT_DESTINATION_ID.into()),
    }
}

#[derive(Debug, From)]
pub enum ChromecastError {
    AppNotFound,
    AppStatusNotFound,
    RustCastError(rust_cast::errors::Error),
}
#[derive(Serialize)]
pub struct ChromecastStatus {
    current_time: Option<f32>,
    #[serde(serialize_with = "serialize_player_state")]
    player_state: PlayerState,
    #[serde(serialize_with = "serialize_idle_reason")]
    idle_reason: Option<IdleReason>,
}

impl From<StatusEntry> for ChromecastStatus {
    fn from(status: StatusEntry) -> Self {
        ChromecastStatus {
            current_time: status.current_time,
            player_state: status.player_state,
            idle_reason: status.idle_reason,
        }
    }
}

pub trait BaseMediaReceiver {
    fn play(&self) -> Result<ChromecastStatus, ChromecastError>;
    fn pause(&self) -> Result<ChromecastStatus, ChromecastError>;
    fn stop(&self) -> Result<ChromecastStatus, ChromecastError>;
    fn cast(&self, url: Url) -> Result<(), ChromecastError>;
    fn get_status(&self) -> Result<ChromecastStatus, ChromecastError>;
}

#[derive(Clone)]
pub struct MediaReceiver {
    ip: IpAddr,
    port: u16,
    dest_id: String,
}

impl BaseMediaReceiver for MediaReceiver {
    fn play(&self) -> Result<ChromecastStatus, ChromecastError> {
        manage(self, ManageCommmand::Play)
    }
    fn pause(&self) -> Result<ChromecastStatus, ChromecastError> {
        manage(self, ManageCommmand::Pause)
    }
    fn stop(&self) -> Result<ChromecastStatus, ChromecastError> {
        manage(self, ManageCommmand::Stop)
    }
    fn cast(&self, url: Url) -> Result<(), ChromecastError> {
        cast(self, url)
    }
    fn get_status(&self) -> Result<ChromecastStatus, ChromecastError> {
        manage(self, ManageCommmand::Status)
    }
}

enum ManageCommmand {
    Play,
    Pause,
    Stop,
    Status,
}

fn manage(
    med: &MediaReceiver,
    command: ManageCommmand,
) -> Result<ChromecastStatus, ChromecastError> {
    let cast_device = CastDevice::connect_without_host_verification(med.ip.to_string(), med.port)?;

    cast_device.connection.connect(med.dest_id.as_str())?;
    cast_device.heartbeat.ping()?;
    let app_to_manage = CastDeviceApp::DefaultMediaReceiver;
    let status = cast_device.receiver.get_status()?;
    let app = status
        .applications
        .iter()
        .find(|app| CastDeviceApp::from_str(app.app_id.as_str()).unwrap() == app_to_manage);

    cast_device.connection.disconnect(med.dest_id.as_str())?;
    match app {
        Some(app) => {
            cast_device.connection.connect(app.transport_id.as_str())?;
            let status = cast_device
                .media
                .get_status(app.transport_id.as_str(), None)?;
            let entry = status.entries.first().unwrap();

            let retstatus = match command {
                ManageCommmand::Play => cast_device
                    .media
                    .play(app.transport_id.as_str(), entry.media_session_id)
                    .map(Into::into)
                    .map_err(ChromecastError::RustCastError),

                ManageCommmand::Pause => cast_device
                    .media
                    .pause(app.transport_id.as_str(), entry.media_session_id)
                    .map(Into::into)
                    .map_err(ChromecastError::RustCastError),

                ManageCommmand::Stop => cast_device
                    .media
                    .stop(app.transport_id.as_str(), entry.media_session_id)
                    .map(Into::into)
                    .map_err(ChromecastError::RustCastError),

                ManageCommmand::Status => Ok(ChromecastStatus::from(entry.clone())),
            };
            cast_device
                .connection
                .disconnect(app.transport_id.as_str())?;
            retstatus
        }
        None => Err(ChromecastError::AppNotFound),
    }
}

fn cast(med: &MediaReceiver, url: Url) -> Result<(), ChromecastError> {
    let cast_device = CastDevice::connect_without_host_verification(med.ip.to_string(), med.port)?;

    // Connect and ping
    cast_device.connection.connect(med.dest_id.as_str())?;
    cast_device.heartbeat.ping()?;

    // Information about cast device.
    let status = cast_device.receiver.get_status()?;
    for i in 0..status.applications.len() {
        println!("{}", status.applications[i].display_name.as_str());
        println!("{}", status.applications[i].app_id.as_str());
        println!("{}", status.applications[i].status_text.as_str());
    }

    // Launch the application
    let app_to_run = &CastDeviceApp::DefaultMediaReceiver;
    let app = cast_device.receiver.launch_app(app_to_run)?;
    cast_device.connection.connect(app.transport_id.as_str())?;

    // Start casting, returns also a status
    cast_device.media.load(
        app.transport_id.as_str(),
        app.session_id.as_str(),
        &Media {
            // http://commondatastorage.googleapis.com/gtv-videos-bucket/big_buck_bunny_1080p.mp4
            content_id: url.to_string(),
            content_type: "".into(),
            stream_type: StreamType::Live, // "buffered"
            duration: None,
            metadata: None,
        },
    )?;

    // Keeps on casting until connection closes
    //
    // Connection closes automatically when the app changes or restarts etc.
    loop {
        match cast_device.receive() {
            Ok(ChannelMessage::Heartbeat(response)) => {
                println!("[Heartbeat] {:?}", response);

                if let HeartbeatResponse::Ping = response {
                    cast_device.heartbeat.pong()?;
                }
            }
            Ok(ChannelMessage::Connection(ConnectionResponse::Close)) => {
                println!("[Close connection]");
                cast_device
                    .connection
                    .disconnect(app.transport_id.as_str())?;
                break;
            }

            Ok(ChannelMessage::Media(MediaResponse::LoadFailed(_)))
            | Ok(ChannelMessage::Media(MediaResponse::LoadCancelled(_))) => {
                println!("[Loading failed]");
                cast_device
                    .connection
                    .disconnect(app.transport_id.as_str())?;
                break;
            }

            Ok(ChannelMessage::Connection(response)) => match response {
                _ => println!("[Connection] {:?}", response),
            },
            Ok(ChannelMessage::Media(MediaResponse::Status(v))) => {
                if let Some(entry) = v.entries.first() {
                    if entry.player_state.to_string() == "IDLE" {
                        cast_device
                            .connection
                            .disconnect(app.transport_id.as_str())?;
                        println!("[Close connection, idling...]");
                        break;
                    }
                }
                println!("[Status] {:?}", v);
            }
            Ok(ChannelMessage::Media(response)) => println!("[Media] {:?}", response),
            Ok(ChannelMessage::Receiver(response)) => println!("[Receiver] {:?}", response),
            Ok(ChannelMessage::Raw(response)) => println!(
                "Support for the following message type is not yet supported: {:?}",
                response
            ),

            Err(error) => println!("Error occurred while receiving message {}", error),
        }
    }
    println!("Close thread!");
    Ok(())
}

fn serialize_player_state<S>(x: &PlayerState, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&x.to_string())
}

fn serialize_idle_reason<S>(x: &Option<IdleReason>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(match x {
        Some(IdleReason::Cancelled) => "CANCELLED",
        Some(IdleReason::Interrupted) => "INTERRUPTED",
        Some(IdleReason::Finished) => "FINISHED",
        Some(IdleReason::Error) => "ERROR",
        _ => "",
    })
}
