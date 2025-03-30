FROM mirror.gcr.io/rust:1.85 AS builder
ARG GIT_COMMIT_HASH
WORKDIR /build
COPY . /build
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /build/target/release/lnb-server /
USER nonroot
CMD [ "/lnb-server", "-c", "/data/config.toml" ]
