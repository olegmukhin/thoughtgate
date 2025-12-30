use axum::{
    routing::post,
    Router,
    response::sse::{Event, Sse},
};
use futures_util::stream::{self, Stream};
use std::{convert::Infallible, net::SocketAddr, time::Duration};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    // Use standard OpenAI endpoint for compatibility
    let app = Router::new().route("/v1/chat/completions", post(mock_chat));
    
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("🤖 ThoughtGate Mock LLM listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| {
            tracing::error!("Failed to bind to {}: {}", addr, e);
            e
        })?;
    
    axum::serve(listener, app).await
        .map_err(|e| {
            tracing::error!("Server error: {}", e);
            e
        })?;
    
    Ok(())
}

async fn mock_chat() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::info!("Received request, simulating thinking...");
    
    // 1. Simulate "Think Time" (500ms TTFB)
    sleep(Duration::from_millis(500)).await;

    // 2. Simulate "Token Streaming" (50 tokens, 10ms intervals)
    let stream = stream::unfold(0, |i| async move {
        if i >= 50 { return None; }
        sleep(Duration::from_millis(10)).await;
        let data = format!("token_{}", i);
        Some((Ok(Event::default().data(data)), i + 1))
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}
