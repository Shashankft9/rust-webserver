FROM rust as builder

RUN cargo new --bin webserver

WORKDIR /webserver

COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release

RUN rm src/*.rs

RUN rm target/release/deps/webserver*

ADD . ./

RUN cargo build --release

RUN cp target/release/webserver ./

FROM debian:buster-slim

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

EXPOSE 8000

COPY --from=builder /webserver/target/release/webserver /server

CMD ["/server"]
