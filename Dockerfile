FROM rust:1-bookworm AS builder

# Install bun
RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:${PATH}"

WORKDIR /app
COPY . .

# Build frontend (embedded into the binary via rust-embed)
RUN cd ui && bun install && bun run b

# Build server
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /app/target/release/server /
EXPOSE 3001
CMD ["/server"]
