FROM mcr.microsoft.com/devcontainers/rust:1.0.9-bookworm as dev

ARG USERNAME=vscode
USER $USERNAME

WORKDIR /home/$USERNAME/workspace

RUN rustup component add rustfmt clippy
RUN cargo install cargo-watch

FROM rust:1.77.2-bookworm as builder

# Cache build dependencies
RUN cargo new --bin app
WORKDIR /app
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src
RUN cargo build --release

FROM debian:12-slim as tini
ENV TINI_VERSION v0.19.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
RUN chmod +x /tini

FROM gcr.io/distroless/cc-debian12:nonroot as runner
COPY --from=tini --chown=nonroot:nonroot /tini /
ENTRYPOINT [ "/tini", "--" ]

COPY --from=builder --chown=nonroot:nonroot /app/target/release/misskey-webp-proxy /
CMD ["/misskey-webp-proxy"]