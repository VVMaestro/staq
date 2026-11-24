use crate::staq::{Staq, Task};
use chrono::DateTime;
use serde::Deserialize;

use std::{
    collections::{HashMap, VecDeque},
    path::Path,
    time::Duration,
};

use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Row, SqlitePool,
};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub const DEFAULT_STAQ_ID: i64 = 0;

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

#[derive(Deserialize)]
pub struct CreateStaqData {
    pub name: String,
    pub description: Option<String>,
}

pub struct CreateTaskData {
    pub name: String,
    pub description: Option<String>,
    pub placement: Placement,
    pub staq_id: Option<i64>,
}

pub struct UpdateTaskData {
    pub id: i64,
    pub staq_id: Option<i64>,
    pub name: String,
    pub description: String,
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
        placement: row.try_get("placement")?,
        staq_id: row.try_get("staq_id")?,
        created_at,
    })
}

fn decode_empty_staq(row: &sqlx::sqlite::SqliteRow) -> Result<Staq, sqlx::Error> {
    Ok(Staq::new(
        row.try_get("id")?,
        row.try_get("name")?,
        row.try_get("description")?,
    ))
}

pub async fn load_staqs(pool: &SqlitePool) -> Result<Vec<Staq>, sqlx::Error> {
    let staq_rows = sqlx::query("SELECT id, name, description FROM staqs")
        .fetch_all(pool)
        .await?;

    let tasks_rows = sqlx::query(
        "SELECT id, name, description, status, placement, staq_id, created_at FROM tasks \
             WHERE status = 'active' ORDER BY position ASC, id ASC",
    )
    .fetch_all(pool)
    .await?;

    let tasks: Vec<Task> = tasks_rows
        .iter()
        .map(decode_task)
        .collect::<Result<Vec<Task>, sqlx::Error>>()?;

    let mut ungrouped_stack_tasks: Vec<Task> = Vec::new();
    let mut ungrouped_queue_tasks: VecDeque<Task> = VecDeque::new();
    let mut stack_tasks_by_staq: HashMap<i64, Vec<Task>> = HashMap::new();
    let mut queue_tasks_by_staq: HashMap<i64, VecDeque<Task>> = HashMap::new();

    for task in tasks {
        match (task.staq_id, task.placement.as_str()) {
            (Some(id), "queue") => {
                queue_tasks_by_staq.entry(id).or_default().push_back(task);
            }
            (Some(id), "stack") => {
                stack_tasks_by_staq.entry(id).or_default().push(task);
            }
            (None, "queue") => ungrouped_queue_tasks.push_back(task),
            (None, "stack") => ungrouped_stack_tasks.push(task),
            (_, value) => {
                return Err(sqlx::Error::Protocol(format!("invalid placement: {value}")));
            }
        }
    }

    let mut staqs = Vec::new();

    staqs.push(Staq::from_parts(
        DEFAULT_STAQ_ID,
        ungrouped_queue_tasks,
        ungrouped_stack_tasks,
        Some("Default staq".into()),
        Some("Default staq".into()),
    ));

    for staq_row in &staq_rows {
        let id: i64 = staq_row.try_get("id")?;
        let staq = Staq::from_parts(
            id,
            queue_tasks_by_staq.remove(&id).unwrap_or_default(),
            stack_tasks_by_staq.remove(&id).unwrap_or_default(),
            staq_row.try_get("name")?,
            staq_row.try_get("description")?,
        );
        staqs.push(staq);
    }

    Ok(staqs)
}

pub async fn create_staq(pool: &SqlitePool, data: &CreateStaqData) -> Result<Staq, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO staqs (name, description) VALUES (?1, ?2) \
        RETURNING id, name, description",
    )
    .bind(&data.name)
    .bind(&data.description)
    .fetch_one(pool)
    .await?;

    decode_empty_staq(&result)
}

pub async fn delete_staq(pool: &SqlitePool, staq_id: i64) -> Result<(), sqlx::Error> {
    let result = sqlx::query("DELETE FROM staqs WHERE id = ?1")
        .bind(staq_id)
        .execute(pool)
        .await?;

    if result.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound);
    }

    Ok(())
}

pub async fn insert_task(pool: &SqlitePool, data: &CreateTaskData) -> Result<Task, sqlx::Error> {
    let name = &data.name;
    let placement = &data.placement.as_str();
    let description = &data.description.as_deref().unwrap_or("");
    let staq_id = &data.staq_id;

    let mut transaction = pool.begin().await?;

    // Compute the position in the write statement to avoid a read/write race.
    let row = sqlx::query(
        "INSERT INTO tasks (name, placement, position, description, staq_id) \
         SELECT ?1, ?2, COALESCE(MAX(position), 0) + 1, ?3, ?4 FROM tasks \
         WHERE status = 'active' AND placement = ?2 AND staq_id IS ?4 \
         RETURNING id, name, placement, description, status, staq_id, created_at",
    )
    .bind(name)
    .bind(placement)
    .bind(description)
    .bind(staq_id)
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

pub async fn update_task(pool: &SqlitePool, data: &UpdateTaskData) -> Result<Task, sqlx::Error> {
    let row = sqlx::query(
        "UPDATE tasks SET name = ?1, description = ?2 \
         WHERE id = ?3 AND status = 'active' AND staq_id IS ?4 \
         RETURNING id, name, placement, description, status, staq_id, created_at",
    )
    .bind(&data.name)
    .bind(&data.description)
    .bind(data.id)
    .bind(data.staq_id)
    .fetch_one(pool)
    .await?;

    decode_task(&row)
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
    use sqlx::{
        migrate::Migrator,
        sqlite::{SqliteConnectOptions, SqlitePoolOptions},
        Row,
    };

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
                 WHERE name IN ('id', 'name', 'description', 'status', 'placement', 'position', 'created_at', 'staq_id')",
            )
            .fetch_one(&pool)
            .await
            .expect("task columns should be readable");
            assert_eq!(task_column_count, 8);

            let staq_column_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM pragma_table_info('staqs') \
                 WHERE name IN ('id', 'name', 'description', 'created_at')",
            )
            .fetch_one(&pool)
            .await
            .expect("staq columns should be readable");
            assert_eq!(staq_column_count, 4);

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

            let staq_id: i64 = sqlx::query_scalar(
                "INSERT INTO staqs (name, description) VALUES ('Work', 'Work tasks') RETURNING id",
            )
            .fetch_one(&pool)
            .await
            .expect("staq should be inserted");

            sqlx::query(
                "INSERT INTO tasks (name, placement, position, staq_id) \
                 VALUES ('Grouped task', 'stack', 1, ?1)",
            )
            .bind(staq_id)
            .execute(&pool)
            .await
            .expect("task should reference an existing staq");

            let invalid_staq = sqlx::query(
                "INSERT INTO tasks (name, placement, position, staq_id) \
                 VALUES ('Invalid staq', 'queue', 2, ?1)",
            )
            .bind(staq_id + 1)
            .execute(&pool)
            .await;
            assert!(matches!(
                invalid_staq,
                Err(sqlx::Error::Database(error)) if error.is_foreign_key_violation()
            ));

            pool.close().await;

            let reopened_pool = initialize(&database_path)
                .await
                .expect("existing database should be reopened");

            let task = sqlx::query(
                "SELECT name, status, placement, position, created_at, staq_id FROM tasks WHERE id = 1",
            )
            .fetch_one(&reopened_pool)
            .await
            .expect("existing task should be preserved");

            assert_eq!(task.get::<String, _>("name"), "First task");
            assert_eq!(task.get::<String, _>("status"), "active");
            assert_eq!(task.get::<String, _>("placement"), "queue");
            assert_eq!(task.get::<i64, _>("position"), 1);
            assert_eq!(task.get::<Option<i64>, _>("staq_id"), None);

            let grouped_staq_id: i64 =
                sqlx::query_scalar("SELECT staq_id FROM tasks WHERE name = 'Grouped task'")
                    .fetch_one(&reopened_pool)
                    .await
                    .expect("task association should be preserved");
            assert_eq!(grouped_staq_id, staq_id);

            let staq = sqlx::query("SELECT name, description FROM staqs WHERE id = ?1")
                .bind(staq_id)
                .fetch_one(&reopened_pool)
                .await
                .expect("staq should be preserved");
            assert_eq!(staq.get::<String, _>("name"), "Work");
            assert_eq!(staq.get::<String, _>("description"), "Work tasks");

            let created_at = task.get::<String, _>("created_at");
            DateTime::parse_from_rfc3339(&created_at)
                .expect("created_at should use the RFC 3339 format");

            let migration_versions: Vec<i64> = sqlx::query_scalar(
                "SELECT version FROM _sqlx_migrations WHERE success = TRUE ORDER BY version",
            )
            .fetch_all(&reopened_pool)
            .await
            .expect("migration history should be readable");
            assert_eq!(migration_versions, [1, 2]);

            reopened_pool.close().await;
        });
    }

    #[test]
    fn upgrades_first_migration_and_preserves_existing_tasks() {
        tauri::async_runtime::block_on(async {
            let directory = tempfile::tempdir().expect("temporary directory should be created");
            let database_path = directory.path().join("staq.sqlite3");
            let migrations = tempfile::tempdir().expect("migration directory should be created");
            std::fs::write(
                migrations.path().join("0001_create_tasks.sql"),
                include_str!("../migrations/0001_create_tasks.sql"),
            )
            .expect("first migration should be copied");

            let pool = SqlitePoolOptions::new()
                .connect_with(
                    SqliteConnectOptions::new()
                        .filename(&database_path)
                        .create_if_missing(true),
                )
                .await
                .expect("legacy database should be opened");
            Migrator::new(migrations.path())
                .await
                .expect("first migration should be loaded")
                .run(&pool)
                .await
                .expect("first migration should be applied");

            let versions: Vec<i64> =
                sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
                    .fetch_all(&pool)
                    .await
                    .expect("legacy migration history should be readable");
            assert_eq!(versions, [1]);
            let staq_id_columns: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name = 'staq_id'",
            )
            .fetch_one(&pool)
            .await
            .expect("legacy task schema should be readable");
            assert_eq!(staq_id_columns, 0);

            sqlx::query(
                "INSERT INTO tasks (id, name, description, status, placement, position, created_at) \
                 VALUES (7, 'Legacy queue', 'Saved description', 'active', 'queue', 5, '2026-01-01T12:00:00Z'), \
                        (9, 'Legacy stack', '', 'completed', 'stack', 2, '2026-01-02T12:00:00Z')",
            )
            .execute(&pool)
            .await
            .expect("legacy tasks should be inserted");
            let original_tasks = sqlx::query(
                "SELECT id, name, description, status, placement, position, created_at \
                 FROM tasks ORDER BY id",
            )
            .fetch_all(&pool)
            .await
            .expect("legacy tasks should be readable");
            pool.close().await;

            let upgraded_pool = initialize(&database_path)
                .await
                .expect("database should be upgraded to the second migration");
            let upgraded_tasks = sqlx::query(
                "SELECT id, name, description, status, placement, position, created_at, staq_id \
                 FROM tasks ORDER BY id",
            )
            .fetch_all(&upgraded_pool)
            .await
            .expect("upgraded tasks should be readable");
            assert_eq!(upgraded_tasks.len(), original_tasks.len());
            for (before, after) in original_tasks.iter().zip(&upgraded_tasks) {
                for column in ["id", "position"] {
                    assert_eq!(before.get::<i64, _>(column), after.get::<i64, _>(column));
                }
                for column in ["name", "description", "status", "placement", "created_at"] {
                    assert_eq!(
                        before.get::<String, _>(column),
                        after.get::<String, _>(column)
                    );
                }
                assert_eq!(after.get::<Option<i64>, _>("staq_id"), None);
            }

            let staqs = super::load_staqs(&upgraded_pool)
                .await
                .expect("legacy tasks should load into the default staq");
            assert_eq!(staqs.len(), 1);
            assert_eq!(staqs[0].id, 0);
            assert_eq!(staqs[0].queue.len(), 1);
            assert_eq!(staqs[0].queue[0].id, 7);
            assert!(staqs[0].stack.is_empty());

            let staq_id: i64 =
                sqlx::query_scalar("INSERT INTO staqs (name) VALUES ('New staq') RETURNING id")
                    .fetch_one(&upgraded_pool)
                    .await
                    .expect("new staq table should be available");
            sqlx::query("UPDATE tasks SET staq_id = ?1 WHERE id = 7")
                .bind(staq_id)
                .execute(&upgraded_pool)
                .await
                .expect("legacy task should support grouping after upgrade");
            upgraded_pool.close().await;

            let reopened_pool = initialize(&database_path)
                .await
                .expect("upgraded database should reopen without reapplying migrations");
            let versions: Vec<i64> = sqlx::query_scalar(
                "SELECT version FROM _sqlx_migrations WHERE success = TRUE ORDER BY version",
            )
            .fetch_all(&reopened_pool)
            .await
            .expect("both migrations should remain recorded");
            assert_eq!(versions, [1, 2]);
            let saved_staq_id: i64 = sqlx::query_scalar("SELECT staq_id FROM tasks WHERE id = 7")
                .fetch_one(&reopened_pool)
                .await
                .expect("task association should survive reopening");
            assert_eq!(saved_staq_id, staq_id);
            reopened_pool.close().await;
        });
    }
}
