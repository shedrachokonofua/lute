FROM rust:1.88-bookworm as builder
WORKDIR /usr/src/app

# Install protobuf compiler
RUN curl -OL https://github.com/protocolbuffers/protobuf/releases/download/v3.17.3/protoc-3.17.3-linux-x86_64.zip && \
  unzip protoc-3.17.3-linux-x86_64.zip -d protoc3 && \
  mv protoc3/bin/* /usr/local/bin/ && \
  mv protoc3/include/* /usr/local/include/ && \
  rm -rf protoc3 protoc-3.17.3-linux-x86_64.zip

COPY core core
COPY proto proto

RUN cargo install --path core

FROM debian:bookworm-slim
RUN apt-get update
RUN apt-get install ca-certificates -y
RUN update-ca-certificates
RUN rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/lute /usr/local/bin/lute
CMD ["lute"]
