# DOCKER SERVICES

Working in conjunction with this tasks there are three **docker containers**:

- **Prometheus and grafana:** used for [metrics](../metrics-endpoints/metrics.md).
- **A postgres database:** this database is used for storing proof requests and proof themselves once generated. It is composed of a single table, `blob_proofs`, it has three columns (besides the pk `id`):
    - `blob_id`
    - `proof`: This field either contains the generated proof of the `blob_id` or is null, the latter case defines the proof request as still queued/pending.
    - `failed`: This is a boolean field which indicates whether the proof generation failed or not. By default it's set to `false`.
