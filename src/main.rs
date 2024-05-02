mod args;
mod client;
mod handler;
mod processor;
mod webp;

use std::sync::Arc;

use args::Args;
use axum::{
    extract,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing, Router,
};
use clap::Parser;
use client::get_client;
use handler::{media_proxy, ProxyConfig, ProxyQuery};
use reqwest::Client;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt as _};

#[tracing::instrument]
async fn proxy_handler(
    extract::State(state): extract::State<Arc<(Client, f32)>>,
    extract::Query(query): extract::Query<ProxyQuery>,
) -> Result<impl IntoResponse, AppError> {
    let config: ProxyConfig = query.try_into()?;
    let client = &state.0;
    let quality_factor = state.1;

    let buf = media_proxy(client, &config).await?;

    let cache_header = (header::CACHE_CONTROL, "max-age=31536000, immutable");

    // TODO:`Content-Security-Policy`および`Content-Disposition`に対応する
    match config.convert_type {
        handler::ConvertType::Badge => Ok((
            [cache_header, (header::CONTENT_TYPE, "image/png")],
            buf.to_png()?,
        )),
        _ => Ok((
            [cache_header, (header::CONTENT_TYPE, "image/webp")],
            buf.to_webp(quality_factor)?,
        )),
    }
}

#[tracing::instrument]
async fn proxy_handler_with_param(
    extract::Path(_image_param): extract::Path<String>,
    state: extract::State<Arc<(Client, f32)>>,
    query: extract::Query<ProxyQuery>,
) -> Result<impl IntoResponse, AppError> {
    proxy_handler(state, query).await
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();
    tracing::info!(
        host = args.host,
        port = args.port,
        "Waiting request at {}:{} ...",
        args.host,
        args.port,
    );
    let shared_state = Arc::new((
        get_client(args.http_proxy.as_deref())?,
        args.quality_factor as f32,
    ));

    let mut cors_layer = tower_http::cors::CorsLayer::new().allow_methods([http::Method::GET]);
    if args.allow_origin.is_empty() {
        cors_layer = cors_layer.allow_origin(tower_http::cors::Any)
    } else {
        cors_layer = cors_layer.allow_origin(args.allow_origin)
    }

    let app = Router::new()
        .route("/health", routing::get(|| async { "Hello world" }))
        .route("/", routing::get(proxy_handler))
        .route("/*param", routing::get(proxy_handler_with_param))
        .with_state(shared_state)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(cors_layer);
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", args.host, args.port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("stack trace: {:#}", self.0);
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
