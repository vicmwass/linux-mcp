//! Owner-scoped PostgreSQL inventory repository.
//!
//! Every query carries both installation and owner predicates. The repository
//! returns the same public not-found result for absent and foreign IDs; callers
//! must not submit an owner from a request body.

use chrono::{DateTime, Utc};
use linux_mcp_contracts::public::ids::{InstallationId, ServerId, ServerRevision, UserId};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("inventory database error")]
    Database(#[source] sqlx::Error),
    #[error("server not found")]
    NotFound,
    #[error("server revision conflict")]
    RevisionConflict { current: Option<u64> },
    #[error("invalid persisted inventory row: {0}")]
    InvalidRow(String),
}

#[derive(Debug, Clone, FromRow)]
pub struct ServerRow {
    pub server_id: Uuid,
    pub installation_id: Uuid,
    pub owner_user_id: Uuid,
    pub display_name: String,
    pub hostname: String,
    pub port: i32,
    pub username: String,
    pub environment: String,
    pub approval_mode: String,
    pub revision: i64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewServerRow {
    pub server_id: Uuid,
    pub installation_id: Uuid,
    pub owner_user_id: Uuid,
    pub display_name: String,
    pub hostname: String,
    pub port: i32,
    pub username: String,
    pub environment: String,
    pub approval_mode: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct PageCursor {
    pub updated_at: DateTime<Utc>,
    pub server_id: Uuid,
}

#[derive(Clone)]
pub struct InventoryRepository {
    pool: PgPool,
}

impl InventoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_owned(
        &self,
        installation_id: InstallationId,
        owner_user_id: UserId,
        limit: u32,
        after: Option<PageCursor>,
    ) -> Result<Vec<ServerRow>, RepositoryError> {
        let limit = i64::from(limit.clamp(1, 100));
        let rows = if let Some(cursor) = after {
            sqlx::query_as::<_, ServerRow>(
                "SELECT server_id, installation_id, owner_user_id, display_name, hostname, port, username, environment, approval_mode, revision, enabled, created_at, updated_at
                 FROM inventory_servers
                 WHERE installation_id = $1 AND owner_user_id = $2
                   AND (updated_at, server_id) > ($3, $4)
                 ORDER BY updated_at, server_id LIMIT $5",
            )
            .bind(installation_id.as_uuid())
            .bind(owner_user_id.as_uuid())
            .bind(cursor.updated_at)
            .bind(cursor.server_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, ServerRow>(
                "SELECT server_id, installation_id, owner_user_id, display_name, hostname, port, username, environment, approval_mode, revision, enabled, created_at, updated_at
                 FROM inventory_servers
                 WHERE installation_id = $1 AND owner_user_id = $2
                 ORDER BY updated_at, server_id LIMIT $3",
            )
            .bind(installation_id.as_uuid())
            .bind(owner_user_id.as_uuid())
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        };
        rows.map_err(RepositoryError::Database)
    }

    pub async fn get_owned(
        &self,
        installation_id: InstallationId,
        owner_user_id: UserId,
        server_id: ServerId,
    ) -> Result<ServerRow, RepositoryError> {
        sqlx::query_as::<_, ServerRow>(
            "SELECT server_id, installation_id, owner_user_id, display_name, hostname, port, username, environment, approval_mode, revision, enabled, created_at, updated_at
             FROM inventory_servers
             WHERE installation_id = $1 AND owner_user_id = $2 AND server_id = $3",
        )
        .bind(installation_id.as_uuid())
        .bind(owner_user_id.as_uuid())
        .bind(server_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(RepositoryError::Database)?
        .ok_or(RepositoryError::NotFound)
    }

    pub async fn insert(&self, row: NewServerRow) -> Result<ServerRow, RepositoryError> {
        sqlx::query_as::<_, ServerRow>(
            "INSERT INTO inventory_servers (server_id, installation_id, owner_user_id, display_name, hostname, port, username, environment, approval_mode, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$10)
             RETURNING server_id, installation_id, owner_user_id, display_name, hostname, port, username, environment, approval_mode, revision, enabled, created_at, updated_at",
        )
        .bind(row.server_id)
        .bind(row.installation_id)
        .bind(row.owner_user_id)
        .bind(row.display_name)
        .bind(row.hostname)
        .bind(row.port)
        .bind(row.username)
        .bind(row.environment)
        .bind(row.approval_mode)
        .bind(row.now)
        .fetch_one(&self.pool)
        .await
        .map_err(RepositoryError::Database)
    }

    /// Updates all mutable fields only when the caller observed the current revision.
    /// Changing any field increments the revision and therefore invalidates old caches.
    pub async fn update_expected(
        &self,
        installation_id: InstallationId,
        owner_user_id: UserId,
        server_id: ServerId,
        expected_revision: ServerRevision,
        display_name: String,
        hostname: String,
        port: i32,
        username: String,
        environment: String,
        approval_mode: String,
        enabled: bool,
        now: DateTime<Utc>,
    ) -> Result<ServerRow, RepositoryError> {
        let expected: i64 = expected_revision
            .get()
            .try_into()
            .map_err(|_| RepositoryError::InvalidRow("revision overflow".into()))?;
        let updated = sqlx::query_as::<_, ServerRow>(
            "UPDATE inventory_servers SET display_name=$4, hostname=$5, port=$6, username=$7, environment=$8, approval_mode=$9, enabled=$10, revision=revision+1, updated_at=$11
             WHERE installation_id=$1 AND owner_user_id=$2 AND server_id=$3 AND revision=$12
             RETURNING server_id, installation_id, owner_user_id, display_name, hostname, port, username, environment, approval_mode, revision, enabled, created_at, updated_at",
        )
        .bind(installation_id.as_uuid()).bind(owner_user_id.as_uuid()).bind(server_id.as_uuid())
        .bind(display_name).bind(hostname).bind(port).bind(username).bind(environment)
        .bind(approval_mode).bind(enabled).bind(now).bind(expected)
        .fetch_optional(&self.pool).await.map_err(RepositoryError::Database)?;
        updated.ok_or(RepositoryError::RevisionConflict { current: None })
    }

    pub async fn disable_expected(
        &self,
        installation_id: InstallationId,
        owner_user_id: UserId,
        server_id: ServerId,
        expected_revision: ServerRevision,
        now: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        let expected: i64 = expected_revision
            .get()
            .try_into()
            .map_err(|_| RepositoryError::InvalidRow("revision overflow".into()))?;
        let result = sqlx::query(
            "UPDATE inventory_servers SET enabled=FALSE, revision=revision+1, updated_at=$5
             WHERE installation_id=$1 AND owner_user_id=$2 AND server_id=$3 AND revision=$4",
        )
        .bind(installation_id.as_uuid())
        .bind(owner_user_id.as_uuid())
        .bind(server_id.as_uuid())
        .bind(expected)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;
        if result.rows_affected() == 1 {
            Ok(())
        } else {
            Err(RepositoryError::RevisionConflict { current: None })
        }
    }
}
