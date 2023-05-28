FROM rust:1.67 as builder
WORKDIR /usr/src/app
COPY ./core core
COPY ./proto proto

RUN cargo install --path core

FROM debian:bullseye-slim
RUN apt-get update && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/core /usr/local/bin/core
CMD ["core"]