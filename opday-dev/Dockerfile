FROM rust:1.77-slim-buster

WORKDIR /usr/src/opday-dev

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

RUN cargo fetch # this should download dependencies
COPY src/ ./src/
