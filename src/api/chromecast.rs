use crate::AppState;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::Arc;
use url::Url;

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

#[derive(Deserialize, Clone, Debug)]
pub struct ChromecastCastRequest {
    url: Url,
}

pub struct ChromecastApi {
    pub state: Arc<AppState>,
    pub request: ChromecastRequest,
}

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

    pub async fn cast(&self, cast_request: ChromecastCastRequest) -> ApiResponse<()> {
        let state = self.state.clone();
        let receiver = self.get_receiver();
        let url = cast_request.url;
        tokio::spawn(async move {
            match receiver.cast(url) {
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
