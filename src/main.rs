mod client;
mod handler;
mod processor;
mod webp;

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing, Router,
};
use client::get_client;
use handler::{media_proxy, ProxyConfig, ProxyQuery};
use reqwest::Client;

async fn proxy_handler(
    State(client): State<Arc<Client>>,
    Query(query): Query<ProxyQuery>,
) -> Result<impl IntoResponse, AppError> {
    let config: ProxyConfig = query.try_into()?;

    let buf = media_proxy(&client, &config).await?;

    // `Content-Security-Policy`および`Content-Disposition`は未対応
    Ok((
        [(header::CACHE_CONTROL, "max-age=31536000, immutable")],
        buf,
    ))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Arc::new(get_client(None)?);

    let app = Router::new()
        .route("/", routing::get(|| async { "Hello world" }))
        .route("/proxy", routing::get(proxy_handler))
        .with_state(client);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CACHE_CONTROL, "max-age=300")],
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
