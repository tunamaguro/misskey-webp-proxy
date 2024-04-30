mod client;
mod processor;
mod webp;

use serde::Deserialize;
use axum::{routing, Router};
use client::{download_image, get_client, get_image_ext};
use processor::*;



#[tokio::main]
async fn main() {
    let app = Router::new().route("/", routing::get(|| async { "Hello world" }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
