use crate::database::{self, CreateStaqData, CreateTaskData, Placement, UpdateTaskData};
use crate::staq::Staq;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::fmt::Display;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock};

pub const MAX_TASK_DESCRIPTION_LEN: usize = 2000;

#[derive(Debug)]
pub enum AppStateError {
    Database(sqlx::Error),
    StaqNotFound(i64),
    TaskNotFound(i64),
    DefaultStaqDeletion,
    EmptyStaqName,
    EmptyTaskName,
    TaskDescriptionTooLong(usize),
}

impl Display for AppStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(error) => write!(f, "Database error: {error}"),
            Self::StaqNotFound(id) => {
                write!(f, "Staq with id {id} not found in memory")
            }
            Self::TaskNotFound(id) => {
                write!(f, "Task with id {id} not found")
            }
            Self::DefaultStaqDeletion => {
                write!(f, "Cannot delete default staq")
            }
            Self::EmptyStaqName => {
                write!(f, "Staq name must not be empty")
            }
            Self::EmptyTaskName => {
                write!(f, "Task name must not be empty")
            }
            Self::TaskDescriptionTooLong(limit) => {
                write!(f, "Task description must not exceed {limit} characters")
            }
        }
    }
}

struct NormalizedTaskFields {
    name: String,
    description: String,
}

fn normalize_task_fields(
    name: &str,
    description: Option<&str>,
) -> Result<NormalizedTaskFields, AppStateError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppStateError::EmptyTaskName);
    }

    let description = description.unwrap_or("").trim();
    if description.chars().count() > MAX_TASK_DESCRIPTION_LEN {
        return Err(AppStateError::TaskDescriptionTooLong(
            MAX_TASK_DESCRIPTION_LEN,
        ));
    }

    Ok(NormalizedTaskFields {
        name: name.to_owned(),
        description: description.to_owned(),
    })
}

fn db_staq_id(staq_id: i64) -> Option<i64> {
    (staq_id != database::DEFAULT_STAQ_ID).then_some(staq_id)
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

#[derive(Deserialize)]
pub struct TaskUpdateData {
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
        let fields = normalize_task_fields(&task.name, task.description.as_deref())?;

        let staq = self.staq(staq_id).await?;
        let mut staq = staq.lock().await;

        let task_data = CreateTaskData {
            name: fields.name,
            description: Some(fields.description),
            staq_id: db_staq_id(staq.id),
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

    pub async fn update_task(
        &self,
        staq_id: Option<i64>,
        task_id: i64,
        data: &TaskUpdateData,
    ) -> Result<Staq, AppStateError> {
        let fields = normalize_task_fields(&data.name, data.description.as_deref())?;

        let staq = self.staq(staq_id).await?;
        let mut staq = staq.lock().await;

        if staq.find_task(task_id).is_none() {
            return Err(AppStateError::TaskNotFound(task_id));
        }

        let update_data = UpdateTaskData {
            id: task_id,
            staq_id: db_staq_id(staq.id),
            name: fields.name,
            description: fields.description,
        };

        let updated = match database::update_task(&self.pool, &update_data).await {
            Ok(task) => task,
            Err(sqlx::Error::RowNotFound) => return Err(AppStateError::TaskNotFound(task_id)),
            Err(error) => return Err(error.into()),
        };

        staq.update_task(task_id, updated.name, updated.description);

        Ok(staq.clone())
    }

    pub async fn cancel_task(
        &self,
        staq_id: Option<i64>,
        task_id: i64,
    ) -> Result<Staq, AppStateError> {
        let staq = self.staq(staq_id).await?;
        let mut staq = staq.lock().await;

        if staq.find_task(task_id).is_none() {
            return Err(AppStateError::TaskNotFound(task_id));
        }

        match database::cancel_task(&self.pool, task_id, db_staq_id(staq.id)).await {
            Ok(()) => {
                staq.remove_task(task_id);
                Ok(staq.clone())
            }
            Err(sqlx::Error::RowNotFound) => Err(AppStateError::TaskNotFound(task_id)),
            Err(error) => Err(error.into()),
        }
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
