FROM mcr.microsoft.com/devcontainers/rust:1.0.9-bookworm

ARG USERNAME=vscode
USER $USERNAME

WORKDIR /home/$USERNAME/workspace

RUN rustup component add rustfmt clippy
RUN cargo install cargo-watch


