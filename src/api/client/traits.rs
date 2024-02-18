#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use serde::de::DeserializeOwned;

pub(crate) trait RequestTrait {
    type Response;
}

pub(crate) trait JsonFullRequestTrait: RequestTrait {
    type Response: DeserializeOwned;
}

pub(crate) trait ResponseTrait: Sized {}
