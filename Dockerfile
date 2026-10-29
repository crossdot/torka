# syntax=docker/dockerfile:1.7
#
# movie-streamer — production image.
#
# Decisions follow `adr/ADR-006-synology-docker.md` and `adr/ADR-008-hardware-acceleration.md`:
#   * three stages: pinned Rust builder, Node SPA build, then `debian:trixie-slim` runtime with FFmpeg/VA-API;
#   * the builder and the runtime are both Debian trixie, so the binary links against the
#     same glibc the runtime ships (ADR-006 asks for a static musl build; the torrent
#     engine (`librqbit` -> `openssl-sys`) would require a vendored, perl-built OpenSSL
#     for musl, so we pin the glibc pair instead and keep the builder pinned by digest);
#   * primary target is linux/amd64; linux/arm64 is optional and only builds natively
#     (buildx/QEMU), see the note at the end of the file;
#   * no secrets are baked into the image — `APP_API_TOKEN` is supplied at runtime.
#
# Base image digests come from the multi-arch manifests
# (`docker buildx imagetools inspect <image>`) and are bumped deliberately.
# Debian package pins are exact versions available in trixie at the time of writing;
# refresh them together with the digests (see "Refreshing pins" at the end).

ARG RUST_IMAGE=rust:1.99.0-slim@sha256:2752b332db73fdbb7dc576f06c82ed1f312005784ef913d7e04a28f5f55dc581
ARG RUNTIME_IMAGE=debian:trixie-slim@sha256:f6b08b09a05725f9431502d3b5714f5c4576e82c1a84fa1debe0c24a7e0c3ec4
ARG NODE_IMAGE=node:26-alpine

# ---------------------------------------------------------------------------
# Stage 1: build the Rust crate (release profile, trixie -> glibc matched with runtime).
# ---------------------------------------------------------------------------
FROM ${RUST_IMAGE} AS builder

# Native build dependencies for the crate graph:
#   * build-essential/pkg-config/libssl-dev - `openssl-sys` (via `librqbit`);
#   * cmake/perl - `aws-lc-sys` (rustls crypto provider);
#   * libclang-dev - bindgen, used by the TLS/crypto sys crates.
ARG BUILD_DEPS="build-essential cmake pkg-config libssl-dev perl libclang-dev"

ENV CARGO_TERM_COLOR=never \
    CARGO_NET_RETRY=5 \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo

RUN apt-get update \
 && apt-get install -y --no-install-recommends ${BUILD_DEPS} \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Build dependencies first with placeholder sources so that a source-only change
# does not recompile the whole crate graph.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
 && printf 'fn main() {}\n' > src/main.rs \
 && printf '' > src/lib.rs \
 && cargo build --release --locked \
 && rm -rf src

COPY src ./src
# BuildKit preserves source mtimes, which can be older than the placeholder
# artifacts above; cargo fingerprints by mtime, so the sources must be touched
# or it would ship the placeholder binary.
RUN find src -type f -exec touch {} + \
 && cargo build --release --locked \
 && install -D -m 0755 target/release/movie-streamer /out/movie-streamer

# ---------------------------------------------------------------------------
# Stage 1.5: build the Vue SPA (Node). Outputs a static bundle at /build/dist.
# ---------------------------------------------------------------------------
# Pin the Node image by digest in the same way as the Rust/runtime images.
FROM ${NODE_IMAGE} AS frontend

WORKDIR /build
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY frontend/ ./
RUN npm run build

# ---------------------------------------------------------------------------
# Stage 2: runtime.
# ---------------------------------------------------------------------------
FROM ${RUNTIME_IMAGE} AS runtime

# Exact Debian trixie versions; see "Refreshing pins" below.
ARG FFMPEG_VERSION=7:7.1.5-0+deb13u1
ARG LIBVA2_VERSION=2.22.0-3
ARG VAINFO_VERSION=2.22.0+ds1-2
ARG CURL_VERSION=8.14.1-2+deb13u7
ARG INTEL_MEDIA_VERSION=25.2.3+ds1-1

ENV APP_HOST=0.0.0.0 \
    APP_PORT=8080 \
    CACHE_DIR=/cache \
    SESSION_DIR=/cache/sessions \
    STATIC_DIR=/app/dist \
    RUST_LOG=info \
    FFMPEG_PATH=ffmpeg \
    FFPROBE_PATH=ffprobe

# `ffmpeg` ships the GPL build that also provides `ffprobe` (ADR-006: GPL build is
# accepted, so software H.264 via libx264 is available). `libva2`/`vainfo` provide
# the VA-API runtime and its diagnostic tool. `curl` exists only for HEALTHCHECK.
# The Intel media driver lives in `non-free` and is amd64-only, hence the guard;
# arm64 hosts (no Intel iGPU) keep the software path.
RUN sed -i 's/^Components: main$/Components: main non-free non-free-firmware/' \
        /etc/apt/sources.list.d/debian.sources \
 && apt-get update \
 && apt-get install -y --no-install-recommends \
        ffmpeg="${FFMPEG_VERSION}" \
        libva2="${LIBVA2_VERSION}" \
        vainfo="${VAINFO_VERSION}" \
        curl="${CURL_VERSION}" \
 && if [ "$(dpkg --print-architecture)" = "amd64" ]; then \
        apt-get install -y --no-install-recommends \
            intel-media-va-driver-non-free="${INTEL_MEDIA_VERSION}"; \
    fi \
 && rm -rf /var/lib/apt/lists/* /var/cache/apt/archives/*.deb

# Unprivileged runtime user. `/cache` and `/config` are the only writable data
# locations (see compose.yaml); the app never needs root.
RUN groupadd --system --gid 10001 app \
 && useradd --system --uid 10001 --gid app --no-create-home \
        --home-dir /app --shell /usr/sbin/nologin app \
 && install -d -o app -g app -m 0750 /app /cache /cache/sessions /config

COPY --from=builder /out/movie-streamer /usr/local/bin/movie-streamer
COPY --from=frontend --chown=app:app /build/dist /app/dist

WORKDIR /app
USER app

EXPOSE 8080

# Liveness probe; the app installs a SIGTERM handler (see src/main.rs) so the
# container stops gracefully within Docker's default 10s grace period.
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fsS "http://127.0.0.1:${APP_PORT:-8080}/health" -o /dev/null || exit 1

ENTRYPOINT ["/usr/local/bin/movie-streamer"]

# linux/arm64: build natively on an arm64 builder (no cross-compilation, e.g.
# `docker buildx build --platform linux/arm64`), and omit the Intel driver.
#
# Refreshing pins:
#   docker buildx imagetools inspect rust:1.99.0-slim
#   docker buildx imagetools inspect debian:trixie-slim
#   docker run --rm debian:trixie-slim sh -c \
#     'sed -i "s/^Components: main$/Components: main non-free/" \
#        /etc/apt/sources.list.d/debian.sources && apt-get update -qq && \
#      apt-cache policy ffmpeg libva2 vainfo curl intel-media-va-driver-non-free'
