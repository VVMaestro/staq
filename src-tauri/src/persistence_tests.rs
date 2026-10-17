use super::{database, AppState};
use crate::staq::Staq;
use crate::state::{AppStateError, TaskPushData};
use database::Placement::{Queue, Stack};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;

fn task_data(name: &str) -> TaskPushData {
    TaskPushData {
        name: name.into(),
        description: Some(format!("Description of {name}")),
    }
}

async fn insert_staq(pool: &SqlitePool, id: i64, name: &str) {
    sqlx::query("INSERT INTO staqs (id, name, description) VALUES (?1, ?2, ?3)")
        .bind(id)
        .bind(name)
        .bind(format!("Description of {name}"))
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_task(
    pool: &SqlitePool,
    name: &str,
    placement: &str,
    position: i64,
    staq_id: Option<i64>,
) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO tasks (name, description, placement, position, staq_id) \
         VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id",
    )
    .bind(name)
    .bind(format!("Description of {name}"))
    .bind(placement)
    .bind(position)
    .bind(staq_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn staq(state: &AppState, id: i64) -> Staq {
    state.staq(Some(id)).await.unwrap().lock().await.clone()
}

async fn snapshot(state: &AppState) -> HashMap<i64, String> {
    let staqs = state.staqs.read().await;
    let mut result = HashMap::new();
    for (id, staq) in staqs.iter() {
        result.insert(*id, staq.lock().await.serialize());
    }
    result
}

async fn persisted_snapshot(pool: &SqlitePool) -> HashMap<i64, String> {
    database::load_staqs(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|staq| (staq.id, staq.serialize()))
        .collect()
}

fn completion_order(mut staq: Staq) -> Vec<String> {
    let mut names = Vec::new();
    while let Some(task) = staq.pop() {
        names.push(task.name);
    }
    names
}

#[test]
fn restores_tasks_and_order_after_reopening() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("staq.sqlite3");
        let pool = database::initialize(&path).await.unwrap();
        insert_staq(&pool, 1, "First").await;
        insert_staq(&pool, 2, "Second").await;
        for id in [None, Some(1), Some(2)] {
            insert_task(&pool, "A", "queue", 1, id).await;
            insert_task(&pool, "B", "queue", 2, id).await;
            insert_task(&pool, "X", "stack", 1, id).await;
            insert_task(&pool, "Y", "stack", 2, id).await;
        }
        let state = AppState::load(pool).await.unwrap();
        let completed_id = staq(&state, 1).await.peek().unwrap().id;
        state.pop(Some(1)).await.unwrap();
        sqlx::query("UPDATE tasks SET description = 'saved description' WHERE name = 'A'")
            .execute(&state.pool)
            .await
            .unwrap();
        let expected = persisted_snapshot(&state.pool).await;
        state.pool.close().await;
        drop(state);

        let reopened = AppState::load(database::initialize(&path).await.unwrap())
            .await
            .unwrap();
        assert_eq!(snapshot(&reopened).await, expected);
        assert_eq!(reopened.staqs.read().await.len(), 3);
        for id in [0, 1, 2] {
            let staq = staq(&reopened, id).await;
            assert_eq!(staq.queue[0].description, "saved description");
            let expected_order = if id == 1 {
                vec!["X", "A", "B"]
            } else {
                vec!["Y", "X", "A", "B"]
            };
            assert!(staq
                .queue
                .iter()
                .chain(staq.stack.iter())
                .all(|task| task.staq_id == if id == 0 { None } else { Some(id) }));
            assert_eq!(completion_order(staq), expected_order);
        }
        assert_eq!(staq(&reopened, 1).await.name.as_deref(), Some("First"));
        assert_eq!(
            staq(&reopened, 2).await.description.as_deref(),
            Some("Description of Second")
        );
        let status: String = sqlx::query_scalar("SELECT status FROM tasks WHERE id = ?1")
            .bind(completed_id)
            .fetch_one(&reopened.pool)
            .await
            .unwrap();
        assert_eq!(status, "completed");

        let untouched = staq(&reopened, 2).await.serialize();
        let added = reopened
            .push(&task_data("Z"), Stack, Some(1))
            .await
            .unwrap();
        assert_eq!(added.id, 1);
        let new_task = added.peek().unwrap();
        assert!(new_task.id > completed_id);
        assert_eq!(new_task.placement, "stack");
        assert_eq!(new_task.staq_id, Some(1));
        assert_eq!(new_task.description, "Description of Z");
        let mut names = Vec::new();
        for _ in 0..4 {
            names.push(staq(&reopened, 1).await.peek().unwrap().name.clone());
            reopened.pop(Some(1)).await.unwrap();
        }
        assert_eq!(names, ["Z", "X", "A", "B"]);
        assert!(reopened.pop(Some(1)).await.unwrap().peek().is_none());
        assert_eq!(staq(&reopened, 2).await.serialize(), untouched);
        assert_eq!(
            snapshot(&reopened).await,
            persisted_snapshot(&reopened.pool).await
        );
        reopened.pool.close().await;
    });
}

#[test]
fn loads_empty_staqs_with_metadata() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        insert_staq(&pool, 2, "Second").await;
        let state = AppState::load(pool).await.unwrap();
        for (id, name) in [(1, "First"), (2, "Second")] {
            let staq = staq(&state, id).await;
            assert_eq!(staq.id, id);
            assert_eq!(staq.name.as_deref(), Some(name));
            assert_eq!(staq.description, Some(format!("Description of {name}")));
            assert!(staq.queue.is_empty());
            assert!(staq.stack.is_empty());
        }
        assert_eq!(
            snapshot(&state).await,
            persisted_snapshot(&state.pool).await
        );
        state.pool.close().await;
    });
}

#[test]
fn empty_database_has_default_staq() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        let state = AppState::load(pool).await.unwrap();
        assert!(
            state.staqs.read().await.contains_key(&0),
            "default staq must exist even without tasks"
        );
        let default = staq(&state, 0).await;
        assert_eq!(default.id, 0);
        assert!(default.peek().is_none());
        assert!(state.pop(None).await.unwrap().peek().is_none());
        assert!(state.pop(Some(0)).await.unwrap().peek().is_none());
        state.pool.close().await;
    });
}

#[test]
fn omitted_id_and_zero_select_default_without_affecting_other_staqs() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        insert_task(&pool, "A", "queue", 1, None).await;
        insert_task(&pool, "X", "stack", 1, None).await;
        insert_task(&pool, "Other", "queue", 1, Some(1)).await;
        let state = AppState::load(pool).await.unwrap();
        let other = staq(&state, 1).await.serialize();

        let default = state.pop(None).await.unwrap();
        assert_eq!(default.id, 0);
        assert_eq!(default.peek().unwrap().name, "A");
        let default = state.pop(Some(0)).await.unwrap();
        assert_eq!(default.id, 0);
        assert!(default.peek().is_none());
        assert_eq!(staq(&state, 1).await.serialize(), other);
        let completed: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE staq_id IS NULL AND status = 'completed'",
        )
        .fetch_one(&state.pool)
        .await
        .unwrap();
        assert_eq!(completed, 2);
        state.pool.close().await;
    });
}

async fn assert_default_push_survives_reopening(staq_id: Option<i64>) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("staq.sqlite3");
    let pool = database::initialize(&path).await.unwrap();
    insert_staq(&pool, 1, "First").await;
    insert_task(&pool, "Seed", "queue", 1, None).await;
    let state = AppState::load(pool).await.unwrap();
    state.push(&task_data("A"), Queue, staq_id).await.unwrap();
    let default = state.push(&task_data("X"), Stack, staq_id).await.unwrap();
    assert_eq!(default.id, 0);
    assert_eq!(completion_order(default), ["X", "Seed", "A"]);
    let expected = snapshot(&state).await;
    state.pool.close().await;
    drop(state);

    let reopened = AppState::load(database::initialize(&path).await.unwrap())
        .await
        .unwrap();
    assert_eq!(snapshot(&reopened).await, expected);
    assert!(staq(&reopened, 1).await.peek().is_none());
    reopened.pool.close().await;
}

#[test]
fn default_push_with_omitted_id_survives_reopening() {
    tauri::async_runtime::block_on(assert_default_push_survives_reopening(None));
}

#[test]
fn default_push_with_explicit_zero_survives_reopening() {
    tauri::async_runtime::block_on(assert_default_push_survives_reopening(Some(0)));
}

#[test]
fn missing_staq_returns_error_without_changing_memory_or_database() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        insert_task(&pool, "Default", "queue", 1, None).await;
        insert_task(&pool, "Other", "queue", 1, Some(1)).await;
        let state = AppState::load(pool).await.unwrap();
        let before = snapshot(&state).await;
        for placement in [Queue, Stack] {
            assert!(matches!(
                state
                    .push(&task_data("Missing"), placement, Some(999))
                    .await,
                Err(AppStateError::StaqNotFound(999))
            ));
        }
        assert!(matches!(
            state.pop(Some(999)).await,
            Err(AppStateError::StaqNotFound(999))
        ));
        assert_eq!(snapshot(&state).await, before);
        assert_eq!(persisted_snapshot(&state.pool).await, before);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        assert_eq!(count, 2);
        state.pool.close().await;
    });
}

#[test]
fn failed_writes_preserve_memory_and_database() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        insert_staq(&pool, 2, "Second").await;
        insert_task(&pool, "A", "queue", 1, Some(1)).await;
        insert_task(&pool, "Other", "stack", 1, Some(2)).await;
        let state = AppState::load(pool).await.unwrap();
        let expected = snapshot(&state).await;
        sqlx::query("CREATE TRIGGER reject_insert BEFORE INSERT ON tasks BEGIN SELECT RAISE(ABORT, 'test failure'); END")
            .execute(&state.pool).await.unwrap();
        sqlx::query("CREATE TRIGGER reject_update BEFORE UPDATE ON tasks BEGIN SELECT RAISE(ABORT, 'test failure'); END")
            .execute(&state.pool).await.unwrap();
        for placement in [Queue, Stack] {
            assert!(matches!(
                state.push(&task_data("Rejected"), placement, Some(1)).await,
                Err(AppStateError::Database(sqlx::Error::Database(_)))
            ));
        }
        assert!(matches!(
            state.pop(Some(1)).await,
            Err(AppStateError::Database(sqlx::Error::Database(_)))
        ));
        assert_eq!(snapshot(&state).await, expected);
        assert_eq!(persisted_snapshot(&state.pool).await, expected);
        state.pool.close().await;
    });
}

#[test]
fn concurrent_operations_preserve_tasks_in_each_staq() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        insert_staq(&pool, 2, "Second").await;
        let state = Arc::new(AppState::load(pool).await.unwrap());
        let mut handles = Vec::new();
        for id in [1, 2] {
            for index in 0..20 {
                let state = Arc::clone(&state);
                handles.push(tauri::async_runtime::spawn(async move {
                    state
                        .push(&task_data(&format!("Task {id}-{index}")), Queue, Some(id))
                        .await
                        .map(|_| ())
                }));
            }
        }
        let mut first_error = None;
        for handle in handles {
            if let Err(error) = handle.await.unwrap() {
                first_error.get_or_insert(error);
            }
        }
        assert!(
            first_error.is_none(),
            "concurrent push failed: {first_error:?}"
        );
        for id in [1, 2] {
            let staq = staq(&state, id).await;
            assert_eq!(staq.queue.len(), 20);
            assert!(staq.queue.iter().all(|task| task.staq_id == Some(id)));
        }
        assert_eq!(
            snapshot(&state).await,
            persisted_snapshot(&state.pool).await
        );
        for id in [1, 2] {
            let unique_positions: i64 =
                sqlx::query_scalar("SELECT COUNT(DISTINCT position) FROM tasks WHERE staq_id = ?1")
                    .bind(id)
                    .fetch_one(&state.pool)
                    .await
                    .unwrap();
            assert_eq!(unique_positions, 20);
        }
        let mut handles = Vec::new();
        for id in [1, 2] {
            for _ in 0..20 {
                let state = Arc::clone(&state);
                handles.push(tauri::async_runtime::spawn(async move {
                    state.pop(Some(id)).await.unwrap();
                }));
            }
        }
        for handle in handles {
            handle.await.unwrap();
        }
        for id in [1, 2] {
            assert!(staq(&state, id).await.peek().is_none());
        }
        assert_eq!(
            snapshot(&state).await,
            persisted_snapshot(&state.pool).await
        );
        let completed: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status = 'completed'")
                .fetch_one(&state.pool)
                .await
                .unwrap();
        assert_eq!(completed, 40);
        state.pool.close().await;
    });
}

#[test]
fn load_rejects_invalid_dates_and_sorts_each_group_by_position() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        insert_staq(&pool, 2, "Second").await;
        for id in [None, Some(1), Some(2)] {
            insert_task(&pool, "later", "queue", 20, id).await;
            insert_task(&pool, "first", "queue", 5, id).await;
            insert_task(&pool, "tie", "queue", 5, id).await;
            insert_task(&pool, "top", "stack", 20, id).await;
            insert_task(&pool, "bottom", "stack", 5, id).await;
            insert_task(&pool, "middle", "stack", 5, id).await;
        }
        let staqs = database::load_staqs(&pool).await.unwrap();
        assert_eq!(staqs.len(), 3);
        for staq in staqs {
            let names: Vec<_> = staq.queue.iter().map(|task| task.name.as_str()).collect();
            assert_eq!(names, ["first", "tie", "later"]);
            assert_eq!(
                completion_order(staq),
                ["top", "middle", "bottom", "first", "tie", "later"]
            );
        }
        sqlx::query("UPDATE tasks SET created_at = 'invalid' WHERE name = 'first' AND staq_id = 2")
            .execute(&pool)
            .await
            .unwrap();
        assert!(matches!(
            AppState::load(pool.clone()).await,
            Err(sqlx::Error::ColumnDecode { .. })
        ));
        pool.close().await;
    });
}

#[test]
fn missing_active_record_pops_memory_to_match_database() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        insert_staq(&pool, 2, "Second").await;
        let id = insert_task(&pool, "A", "queue", 1, Some(1)).await;
        insert_task(&pool, "B", "queue", 2, Some(1)).await;
        insert_task(&pool, "Other", "queue", 1, Some(2)).await;
        let state = AppState::load(pool).await.unwrap();
        sqlx::query("UPDATE tasks SET status = 'completed' WHERE id = ?1")
            .bind(id)
            .execute(&state.pool)
            .await
            .unwrap();
        let staq = state.pop(Some(1)).await.unwrap();
        assert_eq!(staq.peek().unwrap().name, "B");
        assert_eq!(snapshot(&state).await, persisted_snapshot(&state.pool).await);
        let staq = state.pop(Some(1)).await.unwrap();
        assert!(staq.peek().is_none());
        assert_eq!(snapshot(&state).await, persisted_snapshot(&state.pool).await);
        state.pool.close().await;
    });
}

#[test]
fn push_accepts_missing_description() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        let state = AppState::load(pool).await.unwrap();
        let task = TaskPushData {
            name: "No description".into(),
            description: None,
        };
        let result = state.push(&task, Queue, Some(1)).await.unwrap();
        assert_eq!(result.peek().unwrap().description, "");
        assert_eq!(
            snapshot(&state).await,
            persisted_snapshot(&state.pool).await
        );
        state.pool.close().await;
    });
}

async fn count(pool: &SqlitePool, sql: &'static str, id: i64) -> i64 {
    sqlx::query_scalar(sql).bind(id).fetch_one(pool).await.unwrap()
}

#[test]
fn delete_staq_removes_staq_and_its_tasks_from_memory_and_database() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        insert_staq(&pool, 2, "Second").await;
        insert_task(&pool, "A", "queue", 1, Some(1)).await;
        insert_task(&pool, "X", "stack", 1, Some(1)).await;
        let completed = insert_task(&pool, "Done", "queue", 2, Some(1)).await;
        sqlx::query("UPDATE tasks SET status = 'completed' WHERE id = ?1")
            .bind(completed)
            .execute(&pool)
            .await
            .unwrap();
        insert_task(&pool, "Other", "queue", 1, Some(2)).await;
        insert_task(&pool, "Default", "queue", 1, None).await;
        let state = AppState::load(pool).await.unwrap();
        let other = staq(&state, 2).await.serialize();
        let default = staq(&state, 0).await.serialize();

        state.delete_staq(1).await.unwrap();

        assert!(matches!(
            state.staq(Some(1)).await,
            Err(AppStateError::StaqNotFound(1))
        ));
        assert_eq!(state.staqs.read().await.len(), 2);
        assert_eq!(
            count(&state.pool, "SELECT COUNT(*) FROM staqs WHERE id = ?1", 1).await,
            0
        );
        assert_eq!(
            count(&state.pool, "SELECT COUNT(*) FROM tasks WHERE staq_id = ?1", 1).await,
            0
        );
        assert_eq!(staq(&state, 2).await.serialize(), other);
        assert_eq!(staq(&state, 0).await.serialize(), default);
        assert_eq!(
            snapshot(&state).await,
            persisted_snapshot(&state.pool).await
        );

        for placement in [Queue, Stack] {
            assert!(matches!(
                state.push(&task_data("Late"), placement, Some(1)).await,
                Err(AppStateError::StaqNotFound(1))
            ));
        }
        assert!(matches!(
            state.pop(Some(1)).await,
            Err(AppStateError::StaqNotFound(1))
        ));
        assert!(matches!(
            state.delete_staq(1).await,
            Err(AppStateError::StaqNotFound(1))
        ));
        state.pool.close().await;
    });
}

#[test]
fn delete_rejects_default_and_missing_staqs_without_changes() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        insert_task(&pool, "Default", "queue", 1, None).await;
        insert_task(&pool, "Other", "queue", 1, Some(1)).await;
        let state = AppState::load(pool).await.unwrap();
        let before = snapshot(&state).await;

        assert!(matches!(
            state.delete_staq(database::DEFAULT_STAQ_ID).await,
            Err(AppStateError::DefaultStaqDeletion)
        ));
        assert!(matches!(
            state.delete_staq(999).await,
            Err(AppStateError::StaqNotFound(999))
        ));
        assert_eq!(snapshot(&state).await, before);
        assert_eq!(persisted_snapshot(&state.pool).await, before);
        state.pool.close().await;
    });
}

#[test]
fn failed_delete_preserves_memory_and_database() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        insert_task(&pool, "A", "queue", 1, Some(1)).await;
        let state = AppState::load(pool).await.unwrap();
        let expected = snapshot(&state).await;
        sqlx::query("CREATE TRIGGER reject_delete BEFORE DELETE ON staqs BEGIN SELECT RAISE(ABORT, 'test failure'); END")
            .execute(&state.pool).await.unwrap();

        assert!(matches!(
            state.delete_staq(1).await,
            Err(AppStateError::Database(sqlx::Error::Database(_)))
        ));
        assert_eq!(snapshot(&state).await, expected);
        assert_eq!(persisted_snapshot(&state.pool).await, expected);
        state.pool.close().await;
    });
}

#[test]
fn delete_waits_for_in_flight_staq_operation() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        let state = Arc::new(AppState::load(pool).await.unwrap());

        // Hold the staq lock the way an in-flight push or pop would.
        let in_flight = state.staq(Some(1)).await.unwrap();
        let guard = in_flight.lock().await;
        let delete = tauri::async_runtime::spawn({
            let state = Arc::clone(&state);
            async move { state.delete_staq(1).await }
        });

        // Once delete holds the map write lock it can only be waiting for the staq lock.
        while state.staqs.try_read().is_ok() {
            tokio::task::yield_now().await;
        }
        assert_eq!(
            count(&state.pool, "SELECT COUNT(*) FROM staqs WHERE id = ?1", 1).await,
            1
        );

        drop(guard);
        delete.await.unwrap().unwrap();
        assert_eq!(
            count(&state.pool, "SELECT COUNT(*) FROM staqs WHERE id = ?1", 1).await,
            0
        );
        assert!(!state.staqs.read().await.contains_key(&1));
        state.pool.close().await;
    });
}

#[test]
fn delete_holds_staq_lock_until_database_delete_finishes() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        let state = Arc::new(AppState::load(pool).await.unwrap());
        let in_flight = state.staq(Some(1)).await.unwrap();

        // Block database writes so delete stalls inside the DELETE statement.
        let mut writer = state.pool.acquire().await.unwrap();
        sqlx::query("BEGIN IMMEDIATE")
            .execute(&mut *writer)
            .await
            .unwrap();
        let delete = tauri::async_runtime::spawn({
            let state = Arc::clone(&state);
            async move { state.delete_staq(1).await }
        });
        while state.staqs.try_read().is_ok() {
            tokio::task::yield_now().await;
        }
        for _ in 0..1000 {
            if in_flight.try_lock().is_err() {
                break;
            }
            tokio::task::yield_now().await;
        }

        // An operation that obtained the staq before deletion must not run until it is deleted.
        let late = tauri::async_runtime::spawn({
            let state = Arc::clone(&state);
            let in_flight = Arc::clone(&in_flight);
            async move {
                let _guard = in_flight.lock().await;
                count(&state.pool, "SELECT COUNT(*) FROM staqs WHERE id = ?1", 1).await
            }
        });
        for _ in 0..100 {
            tokio::task::yield_now().await;
        }
        sqlx::query("COMMIT").execute(&mut *writer).await.unwrap();
        drop(writer);

        delete.await.unwrap().unwrap();
        assert_eq!(late.await.unwrap(), 0);
        state.pool.close().await;
    });
}

fn staq_data(name: &str, description: Option<&str>) -> database::CreateStaqData {
    database::CreateStaqData {
        name: name.into(),
        description: description.map(Into::into),
    }
}

async fn staq_count(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM staqs")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[test]
fn create_staq_adds_trimmed_staq_to_memory_and_database() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("staq.sqlite3");
        let pool = database::initialize(&path).await.unwrap();
        insert_staq(&pool, 1, "First").await;
        insert_task(&pool, "Other", "queue", 1, Some(1)).await;
        let state = AppState::load(pool).await.unwrap();
        let other = staq(&state, 1).await.serialize();

        let created = state
            .create_staq(staq_data("  Work  ", Some("Work tasks")))
            .await
            .unwrap();
        assert!(created.id > 1);
        assert_eq!(created.name.as_deref(), Some("Work"));
        assert_eq!(created.description.as_deref(), Some("Work tasks"));
        assert!(created.peek().is_none());
        assert_eq!(staq(&state, created.id).await.serialize(), created.serialize());
        let (name, created_at): (String, String) =
            sqlx::query_as("SELECT name, created_at FROM staqs WHERE id = ?1")
                .bind(created.id)
                .fetch_one(&state.pool)
                .await
                .unwrap();
        assert_eq!(name, "Work");
        chrono::DateTime::parse_from_rfc3339(&created_at)
            .expect("created_at should use the RFC 3339 format");

        state.push(&task_data("A"), Queue, Some(created.id)).await.unwrap();
        let pushed = state
            .push(&task_data("X"), Stack, Some(created.id))
            .await
            .unwrap();
        assert!(pushed
            .queue
            .iter()
            .chain(pushed.stack.iter())
            .all(|task| task.staq_id == Some(created.id)));
        assert_eq!(staq(&state, 1).await.serialize(), other);
        let expected = snapshot(&state).await;
        assert_eq!(persisted_snapshot(&state.pool).await, expected);
        state.pool.close().await;
        drop(state);

        let reopened = AppState::load(database::initialize(&path).await.unwrap())
            .await
            .unwrap();
        assert_eq!(snapshot(&reopened).await, expected);
        assert_eq!(completion_order(staq(&reopened, created.id).await), ["X", "A"]);
        reopened.pool.close().await;
    });
}

#[test]
fn create_staq_without_description_stores_null() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        let state = AppState::load(pool).await.unwrap();

        let created = state.create_staq(staq_data("Work", None)).await.unwrap();
        assert_eq!(created.description, None);
        let description: Option<String> =
            sqlx::query_scalar("SELECT description FROM staqs WHERE id = ?1")
                .bind(created.id)
                .fetch_one(&state.pool)
                .await
                .unwrap();
        assert_eq!(description, None);
        assert_eq!(
            snapshot(&state).await,
            persisted_snapshot(&state.pool).await
        );
        state.pool.close().await;
    });
}

#[test]
fn create_staq_rejects_blank_names_without_changes() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        let state = AppState::load(pool).await.unwrap();
        let before = snapshot(&state).await;

        for name in ["", "   ", "\t\n"] {
            assert!(matches!(
                state.create_staq(staq_data(name, Some("Blank"))).await,
                Err(AppStateError::EmptyStaqName)
            ));
        }
        assert_eq!(snapshot(&state).await, before);
        assert_eq!(persisted_snapshot(&state.pool).await, before);
        assert_eq!(staq_count(&state.pool).await, 1);
        state.pool.close().await;
    });
}

#[test]
fn database_rejects_blank_staq_name() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();

        for name in ["", "   "] {
            let result = sqlx::query("INSERT INTO staqs (name) VALUES (?1)")
                .bind(name)
                .execute(&pool)
                .await;
            assert!(matches!(
                result,
                Err(sqlx::Error::Database(error))
                    if error.kind() == sqlx::error::ErrorKind::CheckViolation
            ));
        }
        assert_eq!(staq_count(&pool).await, 0);
        pool.close().await;
    });
}

#[test]
fn failed_create_preserves_memory_and_database() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        insert_staq(&pool, 1, "First").await;
        let state = AppState::load(pool).await.unwrap();
        let expected = snapshot(&state).await;
        sqlx::query("CREATE TRIGGER reject_staq_insert BEFORE INSERT ON staqs BEGIN SELECT RAISE(ABORT, 'test failure'); END")
            .execute(&state.pool).await.unwrap();

        assert!(matches!(
            state.create_staq(staq_data("Work", None)).await,
            Err(AppStateError::Database(sqlx::Error::Database(_)))
        ));
        assert_eq!(snapshot(&state).await, expected);
        assert_eq!(persisted_snapshot(&state.pool).await, expected);
        state.pool.close().await;
    });
}

#[test]
fn created_staq_ids_are_not_reused_after_deletion() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        let state = AppState::load(pool).await.unwrap();

        let first = state.create_staq(staq_data("First", None)).await.unwrap();
        state.push(&task_data("A"), Queue, Some(first.id)).await.unwrap();
        state.delete_staq(first.id).await.unwrap();
        let second = state.create_staq(staq_data("Second", None)).await.unwrap();

        assert!(second.id > first.id);
        assert!(staq(&state, second.id).await.peek().is_none());
        assert!(matches!(
            state.staq(Some(first.id)).await,
            Err(AppStateError::StaqNotFound(_))
        ));
        assert_eq!(
            snapshot(&state).await,
            persisted_snapshot(&state.pool).await
        );
        state.pool.close().await;
    });
}

#[test]
fn list_staqs_returns_all_staqs_sorted_by_id() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        for (id, name) in [(5, "Fifth"), (2, "Second"), (9, "Ninth")] {
            insert_staq(&pool, id, name).await;
        }
        insert_task(&pool, "A", "queue", 1, Some(2)).await;
        insert_task(&pool, "Default", "stack", 1, None).await;
        let state = AppState::load(pool).await.unwrap();
        let created = state.create_staq(staq_data("Created", None)).await.unwrap();

        let staqs = state.list_staqs().await;
        let ids: Vec<i64> = staqs.iter().map(|staq| staq.id).collect();
        assert_eq!(ids, [0, 2, 5, 9, created.id]);
        let listed: HashMap<i64, String> = staqs
            .iter()
            .map(|staq| (staq.id, staq.serialize()))
            .collect();
        assert_eq!(listed, snapshot(&state).await);
        assert_eq!(listed, persisted_snapshot(&state.pool).await);
        state.pool.close().await;
    });
}
