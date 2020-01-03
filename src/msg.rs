use crate::api;

pub enum NotifyMessage {
    EncodingStarted,
    RequestClosed,
    ErrorDuringCasting(api::ApiError),
}
