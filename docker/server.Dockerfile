FROM rust:1.73 as builder

WORKDIR /

ADD . /

RUN cargo build --release

FROM rust:1.73 as release

COPY --from=builder /target/release/bot_arena /bot_arena

ENTRYPOINT ["/bot_arena"]