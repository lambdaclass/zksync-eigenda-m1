use std::str::FromStr;

use tonic::transport::{Channel, ClientTlsConfig, Endpoint};

use crate::generated::disperser::{self, disperser_client::DisperserClient};

#[derive(Debug, Clone)]
pub struct EigenClientRetriever {
    client: DisperserClient<Channel>,
}

impl EigenClientRetriever {
    pub async fn new(disperser_rpc: &str) -> anyhow::Result<Self> {
        let endpoint = Endpoint::from_str(disperser_rpc)?.tls_config(ClientTlsConfig::new())?;
        let client = DisperserClient::connect(endpoint)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to Disperser server: {}", e))?;

        Ok(EigenClientRetriever { client })
    }

    pub async fn get_blob_data(
        &self,
        blob_index: u32,
        batch_header_hash: Vec<u8>,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let get_response = self
            .client
            .clone()
            .retrieve_blob(disperser::RetrieveBlobRequest {
                batch_header_hash,
                blob_index,
            })
            .await?
            .into_inner();

        if get_response.data.is_empty() {
            return Err(anyhow::anyhow!("Empty data returned from Disperser"));
        }

        let data = kzgpad_rs::remove_empty_byte_from_padded_bytes(&get_response.data);
        Ok(Some(data))
    }
}
