use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use reqwest::{RequestBuilder, Url};
use serde::{Deserialize, Serialize};

use crate::api::client::{ApiError, ApiRequest, PlatformApiRequest};
use crate::codec::Cid;

const NOT_FOUND_HOST_ID: &str = "NA";

#[derive(Serialize)]
pub(crate) struct LocateRequest {
    cids: Vec<String>,
}

impl LocateRequest {
    pub(crate) fn new(cids: Vec<Cid>) -> Self {
        let cids = cids
            .into_iter()
            .map(|c| c.as_base64url_multicodec())
            .collect::<Vec<_>>();

        Self { cids }
    }
}

#[async_trait(?Send)]
impl ApiRequest for LocateRequest {
    type Response = InnerLocateResponse;

    async fn add_payload(
        &mut self,
        request_builder: RequestBuilder,
    ) -> Result<RequestBuilder, ApiError> {
        Ok(request_builder.json(&self))
    }

    fn path(&self) -> String {
        "/api/v1/blocks/locate".to_string()
    }
}

impl PlatformApiRequest for LocateRequest {}

// note(sstelfox): one of the keys is "NA" indicating the blocks couldn't be found, this map is Url
// -> Vec<Cid>.
#[derive(Deserialize)]
pub(crate) struct InnerLocateResponse(HashMap<String, Vec<String>>);

pub struct LocateResponse {
    cid_locations: HashMap<Cid, Vec<usize>>,
    known_missing_blocks: HashSet<Cid>,

    seen_hosts: HashSet<Url>,
    storage_hosts: Vec<Url>,
}

impl LocateResponse {
    pub fn is_missing(&self, cid: &Cid) -> bool {
        self.known_missing_blocks.contains(cid)
    }

    pub fn storage_hosts_with_cid(&self, cid: &Cid) -> Option<Vec<&Url>> {
        let known_indexes = self.cid_locations.get(cid)?;
        let idx_list = known_indexes
            .iter()
            .flat_map(|idx| self.storage_hosts.get(*idx))
            .collect();
        Some(idx_list)
    }
}

impl TryFrom<InnerLocateResponse> for LocateResponse {
    type Error = ApiError;

    fn try_from(value: InnerLocateResponse) -> Result<Self, Self::Error> {
        // todo(sstelfox): might want to use a hashset here to deduplicate but the server
        // should not be duplicating things... trust but verify...
        let mut cid_locations: HashMap<Cid, Vec<usize>> = HashMap::new();
        let mut known_missing_blocks = HashSet::new();

        let mut seen_hosts = HashSet::new();
        let mut storage_hosts = Vec::new();

        for (host_str, cid_list) in value.0 {
            if host_str == NOT_FOUND_HOST_ID {
                for missing_cid in cid_list {
                    let cid = Cid::try_from(missing_cid.as_str()).map_err(|err| {
                        let err_msg = format!("failed to parse CID in missing list: {err}");
                        ApiError::InvalidData(err_msg)
                    })?;

                    known_missing_blocks.insert(cid);
                }

                continue;
            }

            let host_url = Url::parse(&host_str).map_err(|err| {
                let err_msg = format!("failed to parse host URL from CID lookup: {err}");
                ApiError::InvalidData(err_msg)
            })?;

            if !seen_hosts.contains(&host_url) {
                seen_hosts.insert(host_url.clone());
                storage_hosts.push(host_url.clone());
            }

            let host_idx = storage_hosts
                .iter()
                .position(|sh| sh == &host_url)
                .expect("just inserted");

            for cid_str in cid_list.iter() {
                let cid = Cid::try_from(cid_str.as_str()).map_err(|err| {
                    let err_msg = format!("failed to parse CID host {host_url} list: {err}");
                    ApiError::InvalidData(err_msg)
                })?;

                cid_locations.entry(cid).or_default().push(host_idx);
            }
        }

        let resp = Self {
            cid_locations,
            known_missing_blocks,
            seen_hosts,
            storage_hosts,
        };

        Ok(resp)
    }
}
