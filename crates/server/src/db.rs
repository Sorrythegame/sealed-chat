//! SQLite database initialization and schema.

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;

/// Open (creating if needed) the SQLite database and apply the schema.
pub async fn init_pool(database_url: &str) -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    migrate(&pool).await?;
    Ok(pool)
}

pub(crate) async fn migrate(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(SCHEMA).execute(pool).await?;

    // `CREATE TABLE IF NOT EXISTS` does not add new columns to an existing
    // installation. Keep this lightweight migration idempotent for databases
    // created before profiles had a bio field.
    let user_columns = sqlx::query("PRAGMA table_info(users)")
        .fetch_all(pool)
        .await?;
    let has_bio = user_columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "bio");
    if !has_bio {
        sqlx::query("ALTER TABLE users ADD COLUMN bio TEXT NOT NULL DEFAULT ''")
            .execute(pool)
            .await?;
    }

    let invite_columns = sqlx::query("PRAGMA table_info(invite_codes)")
        .fetch_all(pool)
        .await?;
    let has_restricted_username = invite_columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "restricted_username");
    if !has_restricted_username {
        sqlx::query("ALTER TABLE invite_codes ADD COLUMN restricted_username TEXT")
            .execute(pool)
            .await?;
    }

    Ok(())
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS users (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    username      TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    avatar        TEXT,
    bio           TEXT NOT NULL DEFAULT '',
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS devices (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         INTEGER NOT NULL REFERENCES users(id),
    device_name     TEXT NOT NULL,
    public_identity TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_devices_user ON devices(user_id);

CREATE TABLE IF NOT EXISTS conversations (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS conversation_members (
    conversation_id INTEGER NOT NULL REFERENCES conversations(id),
    user_id         INTEGER NOT NULL REFERENCES users(id),
    ephemeral_pub   TEXT,
    UNIQUE(conversation_id, user_id)
);

CREATE TABLE IF NOT EXISTS messages (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id   INTEGER NOT NULL REFERENCES conversations(id),
    sender_device_id  INTEGER NOT NULL REFERENCES devices(id),
    ciphertext        BLOB NOT NULL,
    nonce             BLOB NOT NULL,
    created_at        TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id, id);

CREATE TABLE IF NOT EXISTS attachments (
    id                  TEXT PRIMARY KEY,
    uploader_device_id  INTEGER NOT NULL REFERENCES devices(id),
    ciphertext          BLOB NOT NULL,
    nonce               BLOB NOT NULL,
    mime_type           TEXT NOT NULL,
    size                INTEGER NOT NULL,
    created_at          TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS invite_codes (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    code_hash           TEXT NOT NULL UNIQUE,
    code_prefix         TEXT NOT NULL,
    -- Nullable only for the one-time deployment bootstrap invitation used to
    -- create the initial `wangxin` administrator account.
    created_by_user_id  INTEGER REFERENCES users(id),
    restricted_username TEXT,
    max_uses            INTEGER NOT NULL DEFAULT 1 CHECK(max_uses > 0),
    used_count          INTEGER NOT NULL DEFAULT 0 CHECK(used_count >= 0),
    expires_at          TEXT NOT NULL,
    disabled            INTEGER NOT NULL DEFAULT 0 CHECK(disabled IN (0, 1)),
    created_at          TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_invite_codes_lookup
    ON invite_codes(code_hash, disabled, expires_at);

CREATE TABLE IF NOT EXISTS invite_redemptions (
    invite_code_id  INTEGER NOT NULL REFERENCES invite_codes(id),
    user_id         INTEGER NOT NULL REFERENCES users(id),
    redeemed_at     TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(invite_code_id, user_id)
);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    async fn memory_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("open in-memory sqlite")
    }

    #[tokio::test]
    async fn creates_profile_column_on_a_new_database() {
        let pool = memory_pool().await;

        migrate(&pool).await.expect("run schema migration");

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('users') WHERE name = 'bio'",
        )
        .fetch_one(&pool)
        .await
        .expect("inspect users table");
        assert_eq!(count, 1);

        let invite_tables: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table' AND name IN ('invite_codes', 'invite_redemptions')",
        )
        .fetch_one(&pool)
        .await
        .expect("inspect invite tables");
        assert_eq!(invite_tables, 2);
    }

    #[tokio::test]
    async fn upgrades_a_legacy_users_table_without_losing_data() {
        let pool = memory_pool().await;
        sqlx::query(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                avatar TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&pool)
        .await
        .expect("create legacy users table");
        sqlx::query("INSERT INTO users (username, password_hash) VALUES ('alice', 'hash')")
            .execute(&pool)
            .await
            .expect("insert legacy user");

        migrate(&pool).await.expect("upgrade legacy database");
        migrate(&pool).await.expect("migration remains idempotent");

        let row: (String, String) =
            sqlx::query_as("SELECT username, bio FROM users WHERE username = 'alice'")
                .fetch_one(&pool)
                .await
                .expect("load migrated user");
        assert_eq!(row, ("alice".to_string(), String::new()));
    }

    #[tokio::test]
    async fn adds_restricted_username_to_an_existing_invite_table() {
        let pool = memory_pool().await;
        sqlx::query(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                avatar TEXT,
                bio TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&pool)
        .await
        .expect("create users table");
        sqlx::query(
            "CREATE TABLE invite_codes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code_hash TEXT NOT NULL UNIQUE,
                code_prefix TEXT NOT NULL,
                created_by_user_id INTEGER REFERENCES users(id),
                max_uses INTEGER NOT NULL DEFAULT 1,
                used_count INTEGER NOT NULL DEFAULT 0,
                expires_at TEXT NOT NULL,
                disabled INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&pool)
        .await
        .expect("create legacy invite table");

        migrate(&pool).await.expect("upgrade invite table");
        migrate(&pool).await.expect("migration remains idempotent");

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('invite_codes')
             WHERE name = 'restricted_username'",
        )
        .fetch_one(&pool)
        .await
        .expect("inspect invite columns");
        assert_eq!(count, 1);
    }
}
