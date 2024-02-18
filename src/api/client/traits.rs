#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use serde::de::DeserializeOwned;

pub(crate) trait Request {
    type Response;
}

pub(crate) trait FullRequest: Request {
    type Response: DeserializeOwned;
}

pub(crate) trait StreamingRequest: Request {}

pub(crate) trait FullStreamingRequest: FullRequest {}
