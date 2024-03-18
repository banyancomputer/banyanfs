use async_trait::async_trait;

use crate::codec::Cid;

#[async_trait(?Send)]
pub trait DataStore {
    type Client;

    async fn retrieve(
        &self,
        client: &Self::Client,
        cid: Cid,
    ) -> Result<Option<Vec<u8>>, DataStoreError>;

    async fn store(
        &mut self,
        client: &Self::Client,
        cid: Cid,
        data: Vec<u8>,
    ) -> Result<(), DataStoreError>;

    async fn sync(&mut self, client: &Self::Client) -> Result<(), DataStoreError>;

    async fn unsynced_data_size(&self) -> u64;
}

#[derive(Debug, thiserror::Error)]
pub enum DataStoreError {
    #[error("failed to retrieve block")]
    LookupFailure,

    #[error("failed to store block")]
    StoreFailure,

    #[error("block not available in this data store: {0:?}")]
    UnknownBlock(Cid),
}
