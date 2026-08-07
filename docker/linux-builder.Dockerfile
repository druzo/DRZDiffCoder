# docker/linux-builder.Dockerfile
# Reproducible Ubuntu 24.04 build environment for DRZ Diff.
# Bundles every cross-toolchain needed by scripts/release/*.sh.
# Pushed to ghcr.io/druzo/drzdiff-linux-builder by .github/workflows/docker.yml.
#
# Build:   docker build -f docker/linux-builder.Dockerfile -t drzdiff/linux-builder:dev .
# Run:     docker run --rm -v "$PWD:/src" -w /src drzdiff/linux-builder:dev \
#            ./scripts/release.sh PLATFORMS=linux-x86_64 linux-arm64

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    CARGO_TERM_COLOR=always \
    RUSTUP_HOME=/root/.rustup \
    CARGO_HOME=/root/.cargo \
    PATH=/root/.cargo/bin:/root/.local/bin:$PATH

RUN apt-get update -qq \
 && apt-get install -y --no-install-recommends \
      ca-certificates curl wget git \
      build-essential binutils \
      gcc-mingw-w64-x86-64 binutils-mingw-w64-x86-64 \
      gcc-aarch64-linux-gnu g++-aarch64-linux-gnu binutils-aarch64-linux-gnu \
      dpkg-dev fakeroot \
      rpm rpm-build \
      libfuse2t64 libarchive-tools \
      cmake libssl-dev pkg-config \
      imagemagick \
      python3 python3-pil \
      wixl file \
      unzip zip \
 && rm -rf /var/lib/apt/lists/*

# appimagetool (legacy AppImageKit 13) — supports `appimagetool SRC DEST` CLI
RUN mkdir -p /root/.local/bin \
 && curl -fsSL -o /root/.local/bin/appimagetool \
      https://github.com/AppImage/AppImageKit/releases/download/13/obsolete-appimagetool-x86_64.AppImage \
 && chmod +x /root/.local/bin/appimagetool \
 || echo "WARN: appimagetool download failed (offline build?)"

# Rust stable + cross targets ----------------------------------------------
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal \
 && rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-pc-windows-gnu \
 && cargo install cargo-wix --locked

WORKDIR /src

# entrypoint wraps scripts/release.sh with VERSION + PLATFORMS defaults.
COPY docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["./scripts/release.sh"]