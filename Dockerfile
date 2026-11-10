# Multi-stage image for the APOLO AX API.
FROM rust:1.85-bookworm AS builder
WORKDIR /workspace
COPY . .
RUN cargo build --release -p apolo-ax-api

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates gosu \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home apolo
WORKDIR /app
COPY --from=builder /workspace/target/release/apolo-ax-api /usr/local/bin/apolo-ax-api
COPY docker/entrypoint.sh /usr/local/bin/apolo-entrypoint
RUN chmod 0755 /usr/local/bin/apolo-entrypoint
ENV PORT=10000
EXPOSE 10000
ENTRYPOINT ["/usr/local/bin/apolo-entrypoint"]
