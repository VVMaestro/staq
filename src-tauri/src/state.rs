use crate::database::{self, CreateStaqData, CreateTaskData, Placement};
use crate::staq::Staq;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::fmt::Display;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock};

#[derive(Debug)]
pub enum AppStateError {
    Database(sqlx::Error),
    StaqNotFound(i64),
    DefaultStaqDeletion,
    EmptyStaqName,
}

impl Display for AppStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(error) => write!(f, "Database error: {error}"),
            Self::StaqNotFound(id) => {
                write!(f, "Staq with id {id} not found in memory")
            }
            Self::DefaultStaqDeletion => {
                write!(f, "Cannot delete default staq")
            }
            Self::EmptyStaqName => {
                write!(f, "Staq name must not be empty")
            }
        }
    }
}

impl From<sqlx::Error> for AppStateError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(Deserialize)]
pub struct TaskPushData {
    pub name: String,
    pub description: Option<String>,
}

pub struct AppState {
    pub pool: SqlitePool,
    pub staqs: RwLock<HashMap<i64, Arc<Mutex<Staq>>>>,
}

impl AppState {
    pub async fn staq(&self, staq_id: Option<i64>) -> Result<Arc<Mutex<Staq>>, AppStateError> {
        let id = staq_id.unwrap_or(database::DEFAULT_STAQ_ID);
        self.staqs
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or(AppStateError::StaqNotFound(id))
    }

    pub async fn load(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        let staqs = database::load_staqs(&pool)
            .await?
            .into_iter()
            .map(|staq| (staq.id, Arc::new(Mutex::new(staq))))
            .collect();

        Ok(Self {
            pool,
            staqs: RwLock::new(staqs),
        })
    }

    pub async fn list_staqs(&self) -> Vec<Staq> {
        let staqs = self.staqs.read().await;
        let mut result = Vec::with_capacity(staqs.len());
        for staq in staqs.values() {
            result.push(staq.lock().await.clone());
        }
        // HashMap order is arbitrary; the default staq (id 0) goes first.
        result.sort_by_key(|staq| staq.id);
        result
    }

    pub async fn push(
        &self,
        task: &TaskPushData,
        placement: Placement,
        staq_id: Option<i64>,
    ) -> Result<Staq, AppStateError> {
        let staq = self.staq(staq_id).await?;
        let mut staq = staq.lock().await;

        let task_data = CreateTaskData {
            name: task.name.clone(),
            description: task.description.clone(),
            staq_id: (staq.id != database::DEFAULT_STAQ_ID).then_some(staq.id),
            placement,
        };

        let task = database::insert_task(&self.pool, &task_data).await?;

        match placement {
            Placement::Queue => staq.push(task),
            Placement::Stack => staq.push_on_stack(task),
        }

        Ok(staq.clone())
    }

    pub async fn pop(&self, staq_id: Option<i64>) -> Result<Staq, AppStateError> {
        let staq = self.staq(staq_id).await?;
        let mut staq = staq.lock().await;

        if let Some(task_id) = staq.peek().map(|task| task.id) {
            match database::complete_task(&self.pool, task_id).await {
                Ok(()) | Err(sqlx::Error::RowNotFound) => {
                    staq.pop();
                }
                Err(error) => return Err(error.into()),
            }
        }

        Ok(staq.clone())
    }

    pub async fn create_staq(&self, data: CreateStaqData) -> Result<Staq, AppStateError> {
        let name = data.name.trim();
        if name.is_empty() {
            return Err(AppStateError::EmptyStaqName);
        }

        let data = CreateStaqData {
            name: name.to_owned(),
            ..data
        };

        let staq = database::create_staq(&self.pool, &data).await?;

        let mut staqs = self.staqs.write().await;
        staqs.insert(staq.id, Arc::new(Mutex::new(staq.clone())));

        Ok(staq)
    }

    pub async fn delete_staq(&self, staq_id: i64) -> Result<(), AppStateError> {
        if staq_id == database::DEFAULT_STAQ_ID {
            return Err(AppStateError::DefaultStaqDeletion);
        }

        let mut staqs = self.staqs.write().await;
        let staq = staqs
            .get(&staq_id)
            .cloned()
            .ok_or(AppStateError::StaqNotFound(staq_id))?;
        let _staq = staq.lock().await;

        database::delete_staq(&self.pool, staq_id).await?;
        staqs.remove(&staq_id);

        Ok(())
    }
}
