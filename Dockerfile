FROM rust:1.72-buster

RUN cargo install cross && mkdir /rust

WORKDIR /rust
ENTRYPOINT ["cargo"]
CMD ["build", "--target", "aarch64-unknown-linux-gnu", "--release"]
