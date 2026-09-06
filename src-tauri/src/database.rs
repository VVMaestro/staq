use crate::staq::{Staq, Task};
use chrono::DateTime;
use std::{path::Path, time::Duration};

use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Row, SqlitePool,
};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Clone, Copy)]
pub enum Placement {
    Queue,
    Stack,
}

impl Placement {
    fn as_str(self) -> &'static str {
        match self {
            Self::Queue => "queue",
            Self::Stack => "stack",
        }
    }
}

fn decode_task(row: &sqlx::sqlite::SqliteRow) -> Result<Task, sqlx::Error> {
    let created_at: String = row.try_get("created_at")?;
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .map_err(|error| sqlx::Error::ColumnDecode {
            index: "created_at".into(),
            source: Box::new(error),
        })?
        .to_utc();
    Ok(Task {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        status: row.try_get("status")?,
        created_at,
    })
}

pub async fn load_staq(pool: &SqlitePool) -> Result<Staq, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, name, description, status, placement, created_at FROM tasks \
         WHERE status = 'active' ORDER BY position ASC, id ASC",
    )
    .fetch_all(pool)
    .await?;
    let mut queue = std::collections::VecDeque::new();
    let mut stack = Vec::new();
    for row in rows {
        let task = decode_task(&row)?;
        match row.try_get::<&str, _>("placement")? {
            "queue" => queue.push_back(task),
            "stack" => stack.push(task),
            value => return Err(sqlx::Error::Protocol(format!("invalid placement: {value}"))),
        }
    }
    Ok(Staq::from_parts(queue, stack))
}

pub async fn insert_task(
    pool: &SqlitePool,
    name: &str,
    placement: Placement,
) -> Result<Task, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    // Compute the position in the write statement to avoid a read/write race.
    let row = sqlx::query(
        "INSERT INTO tasks (name, placement, position) \
         SELECT ?1, ?2, COALESCE(MAX(position), 0) + 1 FROM tasks \
         WHERE status = 'active' AND placement = ?2 \
         RETURNING id, name, description, status, created_at",
    )
    .bind(name)
    .bind(placement.as_str())
    .fetch_one(&mut *transaction)
    .await?;
    let task = decode_task(&row)?;
    transaction.commit().await?;
    Ok(task)
}

pub async fn complete_task(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    let result =
        sqlx::query("UPDATE tasks SET status = 'completed' WHERE id = ?1 AND status = 'active'")
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

pub async fn initialize(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    MIGRATOR.run(&pool).await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use sqlx::Row;

    use super::initialize;

    #[test]
    fn initializes_and_reopens_database() {
        tauri::async_runtime::block_on(async {
            let directory = tempfile::tempdir().expect("temporary directory should be created");
            let database_path = directory.path().join("staq.sqlite3");

            let pool = initialize(&database_path)
                .await
                .expect("database should be initialized");

            assert!(database_path.is_file());

            let task_column_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM pragma_table_info('tasks') \
                 WHERE name IN ('id', 'name', 'description', 'status', 'placement', 'position', 'created_at')",
            )
            .fetch_one(&pool)
            .await
            .expect("task columns should be readable");
            assert_eq!(task_column_count, 7);

            let index_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM sqlite_master \
                 WHERE type = 'index' AND name = 'tasks_status_placement_position_idx'",
            )
            .fetch_one(&pool)
            .await
            .expect("task index should be readable");
            assert_eq!(index_count, 1);

            sqlx::query("INSERT INTO tasks (name, placement, position) VALUES (?1, ?2, ?3)")
                .bind("First task")
                .bind("queue")
                .bind(1_i64)
                .execute(&pool)
                .await
                .expect("valid task should be inserted");

            let invalid_status = sqlx::query(
                "INSERT INTO tasks (name, status, placement, position) \
                 VALUES ('Invalid status', 'unknown', 'queue', 2)",
            )
            .execute(&pool)
            .await;
            assert!(invalid_status.is_err());

            let invalid_placement = sqlx::query(
                "INSERT INTO tasks (name, placement, position) \
                 VALUES ('Invalid placement', 'unknown', 2)",
            )
            .execute(&pool)
            .await;
            assert!(invalid_placement.is_err());

            pool.close().await;

            let reopened_pool = initialize(&database_path)
                .await
                .expect("existing database should be reopened");

            let task = sqlx::query(
                "SELECT name, status, placement, position, created_at FROM tasks WHERE id = 1",
            )
            .fetch_one(&reopened_pool)
            .await
            .expect("existing task should be preserved");

            assert_eq!(task.get::<String, _>("name"), "First task");
            assert_eq!(task.get::<String, _>("status"), "active");
            assert_eq!(task.get::<String, _>("placement"), "queue");
            assert_eq!(task.get::<i64, _>("position"), 1);

            let created_at = task.get::<String, _>("created_at");
            DateTime::parse_from_rfc3339(&created_at)
                .expect("created_at should use the RFC 3339 format");

            let migration_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
                .fetch_one(&reopened_pool)
                .await
                .expect("migration history should be readable");
            assert_eq!(migration_count, 1);

            reopened_pool.close().await;
        });
    }
}
