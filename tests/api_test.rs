use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TITLE_ID: AtomicU64 = AtomicU64::new(1);

fn base_url() -> String {
    std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_owned())
}

fn unique_title(prefix: &str) -> String {
    let id = NEXT_TITLE_ID.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{}-{id}", std::process::id())
}

async fn create_todo(client: &Client, title: &str) -> Value {
    let response = client
        .post(format!("{}/todos", base_url()))
        .json(&json!({ "title": title }))
        .send()
        .await
        .expect("server is not running; start it with `cargo run` first");

    assert_eq!(response.status(), StatusCode::CREATED);
    response.json().await.expect("invalid create response")
}

async fn delete_todo(client: &Client, id: i64) {
    let response = client
        .delete(format!("{}/todos/{id}", base_url()))
        .send()
        .await
        .expect("delete request failed");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn create() {
    let client = Client::new();
    let title = unique_title("create-test");

    let todo = create_todo(&client, &title).await;
    println!("CREATE response:\n{}", pretty(&todo));

    assert_eq!(todo["title"], title);
    assert_eq!(todo["completed"], false);
    assert!(todo["id"].is_i64());
}

#[tokio::test]
async fn read() {
    let client = Client::new();
    let created = create_todo(&client, &unique_title("read-test")).await;
    let id = created["id"].as_i64().unwrap();

    let response = client
        .get(format!("{}/todos/{id}", base_url()))
        .send()
        .await
        .expect("read request failed");
    assert_eq!(response.status(), StatusCode::OK);
    let todo: Value = response.json().await.expect("invalid read response");
    println!("READ one response:\n{}", pretty(&todo));
    assert_eq!(todo, created);

    let response = client
        .get(format!("{}/todos", base_url()))
        .send()
        .await
        .expect("list request failed");
    assert_eq!(response.status(), StatusCode::OK);
    let todos: Value = response.json().await.expect("invalid list response");
    println!("READ list response:\n{}", pretty(&todos));
    assert!(
        todos
            .as_array()
            .unwrap()
            .iter()
            .any(|todo| todo["id"] == id)
    );
}

#[tokio::test]
async fn update() {
    let client = Client::new();
    let created = create_todo(&client, &unique_title("update-test")).await;
    let id = created["id"].as_i64().unwrap();
    let new_title = unique_title("updated");

    let response = client
        .put(format!("{}/todos/{id}", base_url()))
        .json(&json!({
            "title": new_title,
            "completed": true
        }))
        .send()
        .await
        .expect("update request failed");
    assert_eq!(response.status(), StatusCode::OK);
    let updated: Value = response.json().await.expect("invalid update response");
    println!("UPDATE response:\n{}", pretty(&updated));

    assert_eq!(updated["id"], id);
    assert_eq!(updated["title"], new_title);
    assert_eq!(updated["completed"], true);
}

#[tokio::test]
async fn delete() {
    let client = Client::new();
    let created = create_todo(&client, &unique_title("delete-test")).await;
    let id = created["id"].as_i64().unwrap();

    delete_todo(&client, id).await;
    println!("DELETE response: 204 No Content (id={id})");

    let response = client
        .get(format!("{}/todos/{id}", base_url()))
        .send()
        .await
        .expect("verification request failed");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap()
}
