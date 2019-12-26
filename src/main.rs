extern crate rust_cast;

use rust_cast::channels::heartbeat::HeartbeatResponse;
use rust_cast::channels::media::{Media, StatusEntry, StreamType};
use rust_cast::channels::receiver::CastDeviceApp;
use rust_cast::{CastDevice, ChannelMessage};

const DEFAULT_DESTINATION_ID: &str = "receiver-0";

fn main() {
    println!("Hello, world!");

    let cast_device = match CastDevice::connect_without_host_verification("192.168.8.106", 8009) {
        Ok(cast_device) => cast_device,
        Err(err) => panic!("Could not establish connection with Cast Device: {:?}", err),
    };

    cast_device
        .connection
        .connect(DEFAULT_DESTINATION_ID.to_string())
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
                content_id: String::from("http://commondatastorage.googleapis.com/gtv-videos-bucket/big_buck_bunny_1080p.mp4"),
                content_type: String::from(""),
                stream_type: StreamType::Buffered, // "buffered"
                duration: None,
                metadata: None,
            },
        ).unwrap();

    loop {
        match cast_device.receive() {
            Ok(ChannelMessage::Heartbeat(response)) => {
                println!("[Heartbeat] {:?}", response);

                if let HeartbeatResponse::Ping = response {
                    cast_device.heartbeat.pong().unwrap();
                }
            }

            Ok(ChannelMessage::Connection(response)) => println!("[Connection] {:?}", response),
            Ok(ChannelMessage::Media(response)) => println!("[Media] {:?}", response),
            Ok(ChannelMessage::Receiver(response)) => println!("[Receiver] {:?}", response),
            Ok(ChannelMessage::Raw(response)) => println!(
                "Support for the following message type is not yet supported: {:?}",
                response
            ),

            Err(error) => println!("Error occurred while receiving message {}", error),
        }
    }
}
