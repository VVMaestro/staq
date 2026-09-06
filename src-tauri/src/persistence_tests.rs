use super::{database, AppState};
use database::Placement::{Queue, Stack};
use std::sync::Arc;

#[test]
fn restores_tasks_and_order_after_reopening() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("staq.sqlite3");
        let state = AppState::load(database::initialize(&path).await.unwrap())
            .await
            .unwrap();
        assert!(state.pop().await.unwrap().peek().is_none());
        state.push("A", Queue).await.unwrap();
        state.push("B", Queue).await.unwrap();
        state.push("X", Stack).await.unwrap();
        let snapshot = state.push("Y", Stack).await.unwrap();
        let completed_id = snapshot.peek().unwrap().id;
        state.pop().await.unwrap();
        sqlx::query("UPDATE tasks SET description = 'saved description' WHERE name = 'A'")
            .execute(&state.pool)
            .await
            .unwrap();
        let expected = database::load_staq(&state.pool).await.unwrap().serialize();
        state.pool.close().await;

        let reopened = AppState::load(database::initialize(&path).await.unwrap())
            .await
            .unwrap();
        assert_eq!(reopened.staq.lock().await.serialize(), expected);
        let status: String = sqlx::query_scalar("SELECT status FROM tasks WHERE id = ?1")
            .bind(completed_id)
            .fetch_one(&reopened.pool)
            .await
            .unwrap();
        assert_eq!(status, "completed");
        let added = reopened.push("Z", Stack).await.unwrap();
        assert!(added.peek().unwrap().id > completed_id);
        let mut names = Vec::new();
        for _ in 0..4 {
            names.push(reopened.staq.lock().await.peek().unwrap().name.clone());
            reopened.pop().await.unwrap();
        }
        assert_eq!(names, ["Z", "X", "A", "B"]);
        assert!(database::load_staq(&reopened.pool)
            .await
            .unwrap()
            .peek()
            .is_none());
        reopened.pool.close().await;
    });
}

#[test]
fn failed_writes_preserve_memory_and_database() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        let state = AppState::load(pool).await.unwrap();
        state.push("A", Queue).await.unwrap();
        let expected = state.staq.lock().await.serialize();
        sqlx::query("CREATE TRIGGER reject_insert BEFORE INSERT ON tasks BEGIN SELECT RAISE(ABORT, 'test failure'); END")
            .execute(&state.pool).await.unwrap();
        sqlx::query("CREATE TRIGGER reject_update BEFORE UPDATE ON tasks BEGIN SELECT RAISE(ABORT, 'test failure'); END")
            .execute(&state.pool).await.unwrap();
        assert!(state.push("B", Queue).await.is_err());
        assert!(state.push("X", Stack).await.is_err());
        assert!(state.pop().await.is_err());
        assert_eq!(state.staq.lock().await.serialize(), expected);
        assert_eq!(
            database::load_staq(&state.pool).await.unwrap().serialize(),
            expected
        );
        state.pool.close().await;
    });
}

#[test]
fn concurrent_operations_preserve_tasks() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        let state = Arc::new(AppState::load(pool).await.unwrap());
        let mut handles = Vec::new();
        for index in 0..20 {
            let state = Arc::clone(&state);
            handles.push(tauri::async_runtime::spawn(async move {
                state.push(&format!("Task {index}"), Queue).await.unwrap();
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
        let snapshot = state.staq.lock().await.clone();
        assert_eq!(snapshot.queue.len(), 20);
        assert_eq!(
            database::load_staq(&state.pool).await.unwrap().serialize(),
            snapshot.serialize()
        );
        let unique_positions: i64 =
            sqlx::query_scalar("SELECT COUNT(DISTINCT position) FROM tasks")
                .fetch_one(&state.pool)
                .await
                .unwrap();
        assert_eq!(unique_positions, 20);
        let mut handles = Vec::new();
        for _ in 0..20 {
            let state = Arc::clone(&state);
            handles.push(tauri::async_runtime::spawn(async move {
                state.pop().await.unwrap();
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
        assert!(state.staq.lock().await.peek().is_none());
        let completed: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status = 'completed'")
                .fetch_one(&state.pool)
                .await
                .unwrap();
        assert_eq!(completed, 20);
        state.pool.close().await;
    });
}

#[test]
fn load_rejects_invalid_dates_and_sorts_by_position() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        sqlx::query("INSERT INTO tasks (name, placement, position) VALUES ('later', 'queue', 20), ('first', 'queue', 5), ('tie', 'queue', 5)")
            .execute(&pool).await.unwrap();
        let staq = database::load_staq(&pool).await.unwrap();
        let names: Vec<_> = staq.queue.iter().map(|task| task.name.as_str()).collect();
        assert_eq!(names, ["first", "tie", "later"]);
        sqlx::query("UPDATE tasks SET created_at = 'invalid' WHERE name = 'first'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(AppState::load(pool.clone()).await.is_err());
        pool.close().await;
    });
}

#[test]
fn missing_active_record_does_not_pop_memory() {
    tauri::async_runtime::block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let pool = database::initialize(&directory.path().join("staq.sqlite3"))
            .await
            .unwrap();
        let state = AppState::load(pool).await.unwrap();
        let before = state.push("A", Queue).await.unwrap().serialize();
        sqlx::query("UPDATE tasks SET status = 'completed'")
            .execute(&state.pool)
            .await
            .unwrap();
        assert!(state.pop().await.is_err());
        assert_eq!(state.staq.lock().await.serialize(), before);
        state.pool.close().await;
    });
}
