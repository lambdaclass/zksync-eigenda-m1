use reqwest::Client;
use serde_json::{json, Value};

pub async fn get_inclusion_data(batch_number: u64, url: String, client: &Client) -> anyhow::Result<Vec<u8>> {
    loop {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "unstable_getDataAvailabilityDetails",
            "params": [batch_number]
        });

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let json_response: Value = response.json().await?;
        let result = json_response
            .get("result")
            .ok_or(anyhow::anyhow!("No result field"))?;
        if result.is_null() {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            continue;
        }
        let inclusion_data = result
            .get("inclusionData")
            .ok_or(anyhow::anyhow!("No inclusionData field"))?;
        if inclusion_data.is_null() {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            continue;
        }
        let inclusion_data = inclusion_data
            .as_array()
            .ok_or(anyhow::anyhow!("inclusionData is not an array"))?;
        let inclusion_data: Vec<u8> = inclusion_data
            .iter()
            .filter_map(|v| v.as_u64().map(|num| num as u8))
            .collect();
        return Ok(inclusion_data);
    }
}
