use reqwest::Client;
use std::time::{Duration, Instant};

struct WorkerResult {
    successful_latencies_micros: Vec<u64>,
    failures: u64,
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .map(|value| {
            value
                .parse::<usize>()
                .unwrap_or_else(|_| panic!("{name} must be a positive integer"))
        })
        .unwrap_or(default)
}

/// Measures throughput against a running server.
///
/// Run with:
/// cargo test --test load_test -- --ignored --nocapture
#[tokio::test(flavor = "multi_thread")]
#[ignore = "load test requires a running server and must be started explicitly"]
async fn read_todos_qps() {
    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_owned());
    let concurrency = env_usize("LOAD_TEST_CONCURRENCY", 500);
    let duration_secs = env_usize("LOAD_TEST_DURATION_SECS", 10);
    assert!(
        concurrency > 0,
        "LOAD_TEST_CONCURRENCY must be greater than 0"
    );
    assert!(
        duration_secs > 0,
        "LOAD_TEST_DURATION_SECS must be greater than 0"
    );

    let url = format!("{base_url}/todos");
    let client = Client::builder()
        .pool_max_idle_per_host(concurrency)
        .build()
        .expect("failed to build HTTP client");

    // Fail early with a useful error if the server is unavailable.
    let warmup = client
        .get(&url)
        .send()
        .await
        .expect("server is not running; start it with `cargo run` first");
    assert!(
        warmup.status().is_success(),
        "warm-up request returned {}",
        warmup.status()
    );
    warmup.bytes().await.expect("failed to read warm-up body");

    let started_at = Instant::now();
    let deadline = started_at + Duration::from_secs(duration_secs as u64);
    let mut workers = Vec::with_capacity(concurrency);

    for _ in 0..concurrency {
        let client = client.clone();
        let url = url.clone();

        workers.push(tokio::spawn(async move {
            let mut successful_latencies_micros = Vec::new();
            let mut failures = 0;

            while Instant::now() < deadline {
                let request_started = Instant::now();
                let succeeded = match client.get(&url).send().await {
                    Ok(response) if response.status().is_success() => {
                        response.bytes().await.is_ok()
                    }
                    _ => false,
                };
                let latency = request_started.elapsed().as_micros() as u64;

                if succeeded {
                    successful_latencies_micros.push(latency);
                } else {
                    failures += 1;
                }
            }

            WorkerResult {
                successful_latencies_micros,
                failures,
            }
        }));
    }

    let mut latencies_micros = Vec::new();
    let mut failed_requests = 0;
    for worker in workers {
        let result = worker.await.expect("load-test worker panicked");
        latencies_micros.extend(result.successful_latencies_micros);
        failed_requests += result.failures;
    }

    let elapsed = started_at.elapsed();
    let successful_requests = latencies_micros.len() as u64;
    let qps = successful_requests as f64 / elapsed.as_secs_f64();
    let average_latency_ms = if successful_requests == 0 {
        0.0
    } else {
        latencies_micros.iter().sum::<u64>() as f64 / successful_requests as f64 / 1_000.0
    };
    latencies_micros.sort_unstable();
    let p50_ms = percentile_millis(&latencies_micros, 0.50);
    let p95_ms = percentile_millis(&latencies_micros, 0.95);
    let p99_ms = percentile_millis(&latencies_micros, 0.99);
    let max_ms = latencies_micros.last().copied().unwrap_or(0) as f64 / 1_000.0;

    println!();
    println!("GET {url}");
    println!("Concurrency:       {concurrency}");
    println!("Elapsed:           {:.2}s", elapsed.as_secs_f64());
    println!("Successful:        {successful_requests}");
    println!("Failed:            {failed_requests}");
    println!("QPS:               {qps:.2}");
    println!("Average latency:   {average_latency_ms:.2}ms");
    println!("p50 latency:       {p50_ms:.2}ms");
    println!("p95 latency:       {p95_ms:.2}ms");
    println!("p99 latency:       {p99_ms:.2}ms");
    println!("Maximum latency:   {max_ms:.2}ms");

    assert!(successful_requests > 0, "no requests succeeded");
    assert_eq!(failed_requests, 0, "some requests failed");

    if let Ok(minimum_qps) = std::env::var("MIN_QPS") {
        let minimum_qps = minimum_qps
            .parse::<f64>()
            .expect("MIN_QPS must be a number");
        assert!(
            qps >= minimum_qps,
            "QPS {qps:.2} was below required minimum {minimum_qps:.2}"
        );
    }

    if let Ok(maximum_p99_ms) = std::env::var("MAX_P99_MS") {
        let maximum_p99_ms = maximum_p99_ms
            .parse::<f64>()
            .expect("MAX_P99_MS must be a number");
        assert!(
            p99_ms <= maximum_p99_ms,
            "p99 latency {p99_ms:.2}ms exceeded maximum {maximum_p99_ms:.2}ms"
        );
    }
}

fn percentile_millis(sorted_micros: &[u64], percentile: f64) -> f64 {
    if sorted_micros.is_empty() {
        return 0.0;
    }

    let rank = (percentile * sorted_micros.len() as f64).ceil() as usize;
    sorted_micros[rank.saturating_sub(1)] as f64 / 1_000.0
}
