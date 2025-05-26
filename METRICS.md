# Prometheus Metrics
The JSON rpc server exposes Prometheus metrics at the `/metrics` endpoint, the returned metrics are returned in JSON format:

```bash
curl -X POST http://127.0.0.1:3030 -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"metrics","params":[],"id":1}'
```

The outputed metrics are in the Prometheus text format, you can use a crate like [`prometheus-parse`](https://crates.io/crates/prometheus-parse) to parse the output:

```rs
use serde_json::json;

const SIDECAR_URL: &str = "http://127.0.0.1:3030";

#[tokio::main]
async fn main() {
    let client = reqwest::Client::new();

    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "metrics",
        "params": [],
        "id": 1
    });

    let response_text = client
        .post(SIDECAR_URL)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let parsed_json: serde_json::Value = serde_json::from_str(&response_text).unwrap();
    let metrics_text = parsed_json["result"].as_str().unwrap_or_default();

    let lines: Vec<_> = metrics_text.lines().map(|s| Ok(s.to_owned())).collect();
    let scrape = prometheus_parse::Scrape::parse(lines.into_iter()).unwrap();
    for sample in scrape.samples {
        println!("{:?}", sample.metric);
        println!("{:?}", sample.value);
    }
}
```