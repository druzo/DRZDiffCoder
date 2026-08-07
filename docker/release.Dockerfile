# docker/release.Dockerfile
# Slim runtime image that bundles a pre-built DRZ Diff release.
# Build:   docker build -f docker/release.Dockerfile -t drzdiff/release:0.1.2 \
#            --build-arg VERSION=0.1.2 .
# Run:     docker run --rm -v /host/path:/data drzdiff/release:0.1.2 /data/left.rs /data/right.rs
#
# This image is purely for sandboxed use of the binary; production users
# install via the .deb / .rpm / .AppImage / .msi / .dmg artifacts.

FROM ubuntu:24.04

ARG VERSION=0.1.2
ENV VERSION=${VERSION}

RUN apt-get update -qq \
 && apt-get install -y --no-install-recommends \
      ca-certificates libgtk-3-0 libxcb-render0 libxcb-shape0 libxcb-xfixes0 \
      libdbus-1-3 libatk1.0-0 libatk-bridge2.0-0 libxkbcommon0 libatspi2.0-0 \
 && rm -rf /var/lib/apt/lists/* \
 && useradd -m -u 1000 drzdiff

COPY --chown=drzdiff:drzdiff releases/${VERSION}/linux-x86_64/drzdiff /usr/local/bin/drzdiff

USER drzdiff
WORKDIR /home/drzdiff

ENTRYPOINT ["/usr/local/bin/drzdiff"]
CMD ["--help"]