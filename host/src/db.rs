use std::sync::Arc;

use anyhow::Result;
use sqlx::{Pool, Postgres, Row};
use tokio::sync::Mutex;

/// Retrieves pending proofs from the database.
/// This function is useful for case the sidecar is restarted
/// some proof were left pending.
pub async fn retrieve_pending_proofs(db_pool: Arc<Mutex<Pool<Postgres>>>) -> Result<Vec<String>> {
    let db_lock = db_pool.lock().await;
    let pending_proofs = sqlx::query(
        r#"
        SELECT BLOB_ID FROM BLOB_PROOFS WHERE PROOF IS NULL ORDER BY ID ASC;
        "#,
    )
    .fetch_all(&*db_lock)
    .await?;

    let mut blob_ids = Vec::new();
    for pending_proof in pending_proofs {
        let blob_id: String = pending_proof.get("blob_id");
        blob_ids.push(blob_id);
    }
    Ok(blob_ids)
}

/// Retrieves the next pending proof from the database.
pub async fn retrieve_next_pending_proof(
    db_pool: Arc<Mutex<Pool<Postgres>>>,
) -> Result<Option<String>> {
    let db_lock = db_pool.lock().await;
    let pending_proof = sqlx::query(
        r#"
        SELECT BLOB_ID FROM BLOB_PROOFS WHERE PROOF IS NULL ORDER BY ID ASC LIMIT 1;
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
pub async fn retrieve_blob_id_proof(
    db_pool: Arc<Mutex<Pool<Postgres>>>,
    blob_id: String,
) -> Option<String> {
    let db_lock = db_pool.lock().await;

    let result = sqlx::query(
        r#"
            SELECT PROOF FROM BLOB_PROOFS
            WHERE BLOB_ID = $1
            "#,
    )
    .bind(blob_id)
    .fetch_optional(&*db_lock)
    .await
    .ok()?
    .map(|row| row.get::<Option<String>, _>("proof"))?;

    result
}

/// Deletes a blob id request from the database.
pub async fn delete_blob_id_request(
    db_pool: Arc<Mutex<Pool<Postgres>>>,
    blob_id: String,
) -> Result<()> {
    let db_lock = db_pool.lock().await;

    sqlx::query(
        r#"
            DELETE FROM BLOB_PROOFS
            WHERE BLOB_ID = $1
            "#,
    )
    .bind(blob_id)
    .execute(&*db_lock)
    .await?;
    Ok(())
}
