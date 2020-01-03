use crate::AppState;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::Arc;

use crate::api::ApiError;
use crate::api::ApiResponse;
use crate::chromecast;
use crate::chromecast::BaseMediaReceiver;
use crate::msg;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChromecastRequest {
    ip: IpAddr,
    port: Option<u16>,
    dest_id: Option<String>,
}

pub struct ChromecastApi {
    pub state: Arc<AppState>,
    pub request: ChromecastRequest,
}

// #[async_trait]
impl ChromecastApi {
    fn get_receiver(&self) -> chromecast::MediaReceiver {
        chromecast::get_default_media_receiver(
            &self.request.ip,
            self.request.port,
            self.request.dest_id.clone(),
        )
    }

    pub async fn pause(&self) -> ApiResponse<()> {
        self.get_receiver()
            .pause()
            .map_err(ApiError::ChromecastError)
    }
    pub async fn play(&self) -> ApiResponse<()> {
        self.get_receiver()
            .play()
            .map_err(ApiError::ChromecastError)
    }
    pub async fn stop(&self) -> ApiResponse<()> {
        self.get_receiver()
            .stop()
            .map_err(ApiError::ChromecastError)
    }
    pub async fn status(&self) -> ApiResponse<chromecast::ChromecastStatus> {
        self.get_receiver()
            .get_status()
            .map_err(ApiError::ChromecastError)
    }

    pub async fn cast(&self) -> ApiResponse<()> {
        let state = self.state.clone();
        let receiver = self.get_receiver();
        tokio::spawn(async move {
            match receiver.cast("http://192.168.8.103:3000/file/encode") {
                Ok(_) => {}
                Err(err) => {
                    (*state)
                        .notifier
                        .send(msg::NotifyMessage::ErrorDuringCasting(err.into()))
                        .unwrap();
                }
            }
        });
        Ok(())
    }
}
