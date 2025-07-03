use std::sync::Arc;

use anyhow::Result;
use sqlx::{Pool, Postgres, Row};
use tokio::sync::Mutex;

/// Retrieves the next pending proof from the database.
pub async fn retrieve_next_pending_proof(
    db_pool: Arc<Mutex<Pool<Postgres>>>,
) -> Result<Option<String>> {
    let db_lock = db_pool.lock().await;
    let pending_proof = sqlx::query(
        r#"
        SELECT BLOB_ID FROM BLOB_PROOFS 
        WHERE PROOF IS NULL
        AND FAILED IS NOT TRUE
        ORDER BY ID ASC LIMIT 1;
        "#,
    )
    .fetch_optional(&*db_lock)
    .await?;

    let blob_id = pending_proof.map(|row| row.get("blob_id"));
    Ok(blob_id)
}

/// Persists the blob proof request in the database.
pub async fn store_blob_proof_request(
    db_pool: Arc<Mutex<Pool<Postgres>>>,
    blob_id: String,
) -> Result<()> {
    let db_lock = db_pool.lock().await;

    sqlx::query(
        r#"
        INSERT INTO BLOB_PROOFS (BLOB_ID)
        VALUES ($1)
        "#,
    )
    .bind(blob_id)
    .execute(&*db_lock)
    .await?;
    Ok(())
}

/// Checks if the blob proof request already exists in the database.
pub async fn proof_request_exists(
    db_pool: Arc<Mutex<Pool<Postgres>>>,
    blob_id: String,
) -> Result<bool> {
    let db_lock = db_pool.lock().await;

    let exists = sqlx::query(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM BLOB_PROOFS WHERE BLOB_ID = $1
        )
        "#,
    )
    .bind(blob_id)
    .fetch_one(&*db_lock)
    .await?
    .get::<bool, _>("exists");

    Ok(exists)
}

/// Stores the blob generated proof in the database.
pub async fn store_blob_proof(
    db_pool: Arc<Mutex<Pool<Postgres>>>,
    blob_id: String,
    proof: String,
) -> Result<()> {
    let db_lock = db_pool.lock().await;

    sqlx::query(
        r#"
        UPDATE BLOB_PROOFS
        SET PROOF = $1
        WHERE BLOB_ID = $2
        "#,
    )
    .bind(proof)
    .bind(blob_id)
    .execute(&*db_lock)
    .await?;
    Ok(())
}

/// Retrieves the blob proof from the database.
/// Returns an Option wrapped by a `Result`
/// that may contain:
/// - `None` if the proof does not exist
/// - `Some((proof, failed))` if the proof exists
///   where proof is an optional `String`, existing only if the proof was generated,
///   and failed is a boolean indicating if the proof generation failed.
///
/// In case the query fails, it returns an `Err`.
pub async fn retrieve_blob_id_proof(
    db_pool: Arc<Mutex<Pool<Postgres>>>,
    blob_id: String,
) -> Result<Option<(Option<String>, bool)>> {
    let db_lock = db_pool.lock().await;

    let row = sqlx::query(
        r#"
            SELECT PROOF, FAILED FROM BLOB_PROOFS
            WHERE BLOB_ID = $1
            "#,
    )
    .bind(blob_id)
    .fetch_optional(&*db_lock)
    .await?;

    let result = row.map(|row| {
        let proof: Option<String> = row.get("proof");
        let failed: bool = row.get("failed");
        (proof, failed)
    });

    Ok(result)
}

/// Marks a blob proof request as failed in the database.
pub async fn mark_blob_proof_request_failed(
    db_pool: Arc<Mutex<Pool<Postgres>>>,
    blob_id: String,
) -> Result<()> {
    let db_lock = db_pool.lock().await;

    sqlx::query(
        r#"
            UPDATE BLOB_PROOFS
            SET FAILED = TRUE
            WHERE BLOB_ID = $1
            "#,
    )
    .bind(blob_id)
    .execute(&*db_lock)
    .await?;
    Ok(())
}
