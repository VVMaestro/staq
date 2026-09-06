use crate::database;
use crate::staq::Staq;
use sqlx::SqlitePool;
use tokio::sync::Mutex;

pub struct AppState {
    pub pool: SqlitePool,
    pub staq: Mutex<Staq>,
}

impl AppState {
    pub async fn load(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        let staq = database::load_staq(&pool).await?;
        Ok(Self {
            pool,
            staq: Mutex::new(staq),
        })
    }

    pub async fn push(
        &self,
        name: &str,
        placement: database::Placement,
    ) -> Result<Staq, sqlx::Error> {
        let mut staq = self.staq.lock().await;
        let task = database::insert_task(&self.pool, name, placement).await?;
        match placement {
            database::Placement::Queue => staq.push(task),
            database::Placement::Stack => staq.push_on_stack(task),
        }
        Ok(staq.clone())
    }

    pub async fn pop(&self) -> Result<Staq, sqlx::Error> {
        let mut staq = self.staq.lock().await;
        if let Some(task) = staq.peek() {
            database::complete_task(&self.pool, task.id).await?;
            staq.pop();
        }
        Ok(staq.clone())
    }
}
