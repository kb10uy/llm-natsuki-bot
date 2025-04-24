FROM mirror.gcr.io/rust:1.86 AS builder
ARG GIT_COMMIT_HASH
WORKDIR /build
RUN mkdir .cargo && printf '[target.x86_64-unknown-linux-gnu]\nrustflags = ["-C", "link-arg=-fuse-ld=mold"]\n' > .cargo/config.toml
RUN apt-get update && apt-get install mold
COPY . /build
RUN cargo build --release

FROM mirror.gcr.io/debian:bookworm-slim
RUN apt-get update && apt-get install -y openssl ca-certificates
COPY --from=builder /build/target/release/lnb-server /
USER 1000:1000
CMD [ "/lnb-server", "-c", "/data/config.generated.json" ]
