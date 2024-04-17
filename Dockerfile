FROM mcr.microsoft.com/devcontainers/rust:1.0.9-bookworm

RUN rustup component add rustfmt clippy
RUN cargo install cargo-watch

ARG USERNAME=vscode

WORKDIR /home/$USERNAME/workspace
