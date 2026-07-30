use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{str::FromStr, time::Duration};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Todo {
    pub id: i64,
    pub title: String,
    pub completed: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateTodo {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTodo {
    pub title: String,
    pub completed: bool,
}

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    NotFound,
    Database(sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::NotFound => (StatusCode::NOT_FOUND, "todo not found".to_owned()),
            Self::Database(error) => {
                eprintln!("database error: {error}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_owned(),
                )
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

pub async fn create_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(if database_url.contains(":memory:") {
            1
        } else {
            5
        })
        .connect_with(options)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS todos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            completed INTEGER NOT NULL DEFAULT 0
        )
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub fn create_router(pool: SqlitePool) -> Router {
    Router::new()
        .route("/todos", post(create_todo).get(list_todos))
        .route(
            "/todos/{id}",
            get(get_todo).put(update_todo).delete(delete_todo),
        )
        .with_state(pool)
}

async fn create_todo(
    State(pool): State<SqlitePool>,
    Json(input): Json<CreateTodo>,
) -> Result<(StatusCode, Json<Todo>), AppError> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(AppError::BadRequest("title cannot be empty".to_owned()));
    }

    let result = sqlx::query("INSERT INTO todos (title) VALUES (?)")
        .bind(title)
        .execute(&pool)
        .await?;
    let todo = find_todo(&pool, result.last_insert_rowid()).await?;
    Ok((StatusCode::CREATED, Json(todo)))
}

async fn list_todos(State(pool): State<SqlitePool>) -> Result<Json<Vec<Todo>>, AppError> {
    let todos = sqlx::query_as::<_, Todo>("SELECT id, title, completed FROM todos ORDER BY id")
        .fetch_all(&pool)
        .await?;
    Ok(Json(todos))
}

async fn get_todo(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<Todo>, AppError> {
    Ok(Json(find_todo(&pool, id).await?))
}

async fn update_todo(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateTodo>,
) -> Result<Json<Todo>, AppError> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(AppError::BadRequest("title cannot be empty".to_owned()));
    }

    let result = sqlx::query("UPDATE todos SET title = ?, completed = ? WHERE id = ?")
        .bind(title)
        .bind(input.completed)
        .bind(id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(find_todo(&pool, id).await?))
}

async fn delete_todo(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM todos WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn find_todo(pool: &SqlitePool, id: i64) -> Result<Todo, AppError> {
    sqlx::query_as::<_, Todo>("SELECT id, title, completed FROM todos WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}
