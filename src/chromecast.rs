extern crate rust_cast;

use rust_cast::channels::connection::ConnectionResponse;
use rust_cast::channels::heartbeat::HeartbeatResponse;
use rust_cast::channels::media::{Media, StatusEntry, StreamType};
use rust_cast::channels::receiver::CastDeviceApp;
use rust_cast::{CastDevice, ChannelMessage};

use std::str::FromStr;

const DEFAULT_DESTINATION_ID: &str = "receiver-0";
const DEFAULT_PORT: u16 = 8009;

/*
let rec = get_default_media_receiver("192.168.8.106")
rec.cast("http://commondatastorage.googleapis.com/gtv-videos-bucket/big_buck_bunny_1080p.mp4");

rec.pause();
rec.play();
// reciever.stop();
*/

pub trait BaseMediaReceiver {
    fn play(&self);
    fn pause(&self);
    fn stop(&self);
    fn cast(&self, url: &str);
    fn get_current_time(&self) -> Option<f32>;
    // fn get_status(&self) -> Option<StatusEntry>;
}

pub struct MediaReceiver {
    ip: String,
    port: u16,
    dest_id: String,
}

pub fn get_default_media_receiver(ip: &str) -> Box<dyn BaseMediaReceiver> {
    Box::new(MediaReceiver {
        ip: ip.into(),
        port: DEFAULT_PORT,
        dest_id: DEFAULT_DESTINATION_ID.into(),
    })
}

impl BaseMediaReceiver for MediaReceiver {
    fn play(&self) {
        manage(self, ManageCommmand::Play);
    }
    fn pause(&self) {
        manage(self, ManageCommmand::Pause);
    }
    fn stop(&self) {
        manage(self, ManageCommmand::Stop);
    }
    fn cast(&self, url: &str) {
        cast(self, url);
    }
    fn get_current_time(&self) -> Option<f32> {
        get_current_time(self)
    }
}

enum ManageCommmand {
    Play,
    Pause,
    Stop,
}

fn manage(med: &MediaReceiver, command: ManageCommmand) {
    let cast_device = match CastDevice::connect_without_host_verification(med.ip.as_str(), med.port)
    {
        Ok(cast_device) => cast_device,
        Err(err) => panic!("Could not establish connection with Cast Device: {:?}", err),
    };

    cast_device
        .connection
        .connect(med.dest_id.as_str())
        .unwrap();
    cast_device.heartbeat.ping().unwrap();
    let app_to_manage = CastDeviceApp::DefaultMediaReceiver;
    let status = cast_device.receiver.get_status().unwrap();
    let app = status
        .applications
        .iter()
        .find(|app| CastDeviceApp::from_str(app.app_id.as_str()).unwrap() == app_to_manage);
    match app {
        Some(app) => {
            cast_device
                .connection
                .connect(app.transport_id.as_str())
                .unwrap();
            let status = cast_device
                .media
                .get_status(app.transport_id.as_str(), None)
                .unwrap();
            let status = status.entries.first().unwrap();

            match command {
                ManageCommmand::Play => {
                    cast_device
                        .media
                        .play(app.transport_id.as_str(), status.media_session_id)
                        .unwrap();
                }

                ManageCommmand::Pause => {
                    cast_device
                        .media
                        .pause(app.transport_id.as_str(), status.media_session_id)
                        .unwrap();
                }

                ManageCommmand::Stop => {
                    cast_device
                        .media
                        .stop(app.transport_id.as_str(), status.media_session_id)
                        .unwrap();
                }
            }
        }
        None => {
            println!("manage: App not found");
        }
    }
}

fn get_current_time(med: &MediaReceiver) -> Option<f32> {
    let cast_device = match CastDevice::connect_without_host_verification(med.ip.as_str(), med.port)
    {
        Ok(cast_device) => cast_device,
        Err(err) => panic!("Could not establish connection with Cast Device: {:?}", err),
    };

    cast_device
        .connection
        .connect(med.dest_id.as_str())
        .unwrap();
    cast_device.heartbeat.ping().unwrap();
    let app_to_manage = CastDeviceApp::DefaultMediaReceiver;
    let status = cast_device.receiver.get_status().unwrap();
    let app = status
        .applications
        .iter()
        .find(|app| CastDeviceApp::from_str(app.app_id.as_str()).unwrap() == app_to_manage);

    match app {
        Some(app) => {
            cast_device
                .connection
                .connect(app.transport_id.as_str())
                .unwrap();
            let status = cast_device
                .media
                .get_status(app.transport_id.as_str(), None)
                .unwrap();
            // let statusfirst = .unwrap();
            // Some(StatusEntry::new(statusfirst))
            // Some(statusfirst)
            let stentry = status.entries.first().unwrap();
            stentry.current_time
        }
        None => None,
    }
}

fn cast(med: &MediaReceiver, url: &str) {
    let cast_device = match CastDevice::connect_without_host_verification(med.ip.as_str(), med.port)
    {
        Ok(cast_device) => cast_device,
        Err(err) => panic!("Could not establish connection with Cast Device: {:?}", err),
    };

    cast_device
        .connection
        .connect(med.dest_id.as_str())
        .unwrap();
    cast_device.heartbeat.ping().unwrap();

    // Information about cast device.
    let status = cast_device.receiver.get_status().unwrap();
    for i in 0..status.applications.len() {
        println!("{}", status.applications[i].display_name.as_str());
        println!("{}", status.applications[i].app_id.as_str());
        println!("{}", status.applications[i].status_text.as_str());
    }

    let app_to_run = &CastDeviceApp::DefaultMediaReceiver;
    let app = cast_device.receiver.launch_app(app_to_run).unwrap();
    cast_device
        .connection
        .connect(app.transport_id.as_str())
        .unwrap();
    let media = cast_device
        .media
        .load(
            app.transport_id.as_str(),
            app.session_id.as_str(),
            &Media {
                // http://commondatastorage.googleapis.com/gtv-videos-bucket/big_buck_bunny_1080p.mp4
                content_id: url.into(),
                content_type: "".into(),
                stream_type: StreamType::Live, // "buffered"
                duration: None,
                metadata: None,
            },
        )
        .unwrap();

    // Keeps on casting until connection closes
    //
    // Connection closes automatically when the app changes or restarts etc.
    loop {
        match cast_device.receive() {
            Ok(ChannelMessage::Heartbeat(response)) => {
                println!("[Heartbeat] {:?}", response);

                if let HeartbeatResponse::Ping = response {
                    cast_device.heartbeat.pong().unwrap();
                }
            }

            Ok(ChannelMessage::Connection(response)) => match response {
                ConnectionResponse::Close => {
                    println!("[Close connection]");
                    break;
                }
                _ => println!("[Connection] {:?}", response),
            },
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
}
