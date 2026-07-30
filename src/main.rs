use axum_backend::{create_pool, create_router};
use tokio::net::TcpListener;
use tower::limit::ConcurrencyLimitLayer;

#[tokio::main]
async fn main() {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://app.db".to_owned());
    let max_concurrent_requests = std::env::var("MAX_CONCURRENT_REQUESTS")
        .map(|value| {
            value
                .parse::<usize>()
                .expect("MAX_CONCURRENT_REQUESTS must be a positive integer")
        })
        .unwrap_or(200);
    assert!(
        max_concurrent_requests > 0,
        "MAX_CONCURRENT_REQUESTS must be greater than zero"
    );

    let pool = create_pool(&database_url)
        .await
        .expect("failed to initialize database");
    let app = create_router(pool).layer(ConcurrencyLimitLayer::new(max_concurrent_requests));

    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind port 3000");
    println!(
        "server listening on http://localhost:3000 (max {max_concurrent_requests} concurrent requests)"
    );
    axum::serve(listener, app).await.expect("server failed");
}
