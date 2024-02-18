#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use serde::de::DeserializeOwned;

pub(crate) trait ApiRequestTrait {
    type Response: ApiResponseTrait + DeserializeOwned;
}

pub(crate) trait ApiResponseTrait: Sized {}
