# docker/ — DRZ Diff container images

## linux-builder

Reproducible Ubuntu 24.04 build environment. Pre-installs:

- rustup stable + cross targets (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-pc-windows-gnu`)
- `gcc-mingw-w64-x86-64`, `gcc-aarch64-linux-gnu` (cross compilers)
- `dpkg-dev`, `fakeroot` (`.deb` packaging)
- `rpm`, `rpm-build` (`.rpm` packaging — new in v0.1.2)
- `appimagetool` (legacy AppImageKit 13)
- `imagemagick`, `python3-pil` (icon generation)
- `wixl` (Linux-native WiX alternative)
- `libdmg-hfsplus` prereqs (`.dmg` generation)
- `cargo-wix` (`.msi` authoring)

### Build

```bash
docker build -f docker/linux-builder.Dockerfile -t drzdiff/linux-builder:dev .
```

### Run locally

```bash
docker run --rm -v "$PWD:/src" -w /src drzdiff/linux-builder:dev \
  ./scripts/release.sh PLATFORMS=linux-x86_64 linux-arm64
```

### Pushed image

`ghcr.io/druzo/drzdiff-linux-builder:0.1.2` — built automatically by
`.github/workflows/docker.yml` on every push to `main`.

## release

Slim runtime image wrapping a pre-built `drzdiff` binary. Used for sandboxed
CLI execution. Production users should install via the packaged artifacts.

```bash
docker build -f docker/release.Dockerfile -t drzdiff/release:0.1.2 \
  --build-arg VERSION=0.1.2 .
docker run --rm -v "$PWD/data:/data" drzdiff/release:0.1.2 \
  /data/left.rs /data/right.rs
```

## entrypoint.sh

Resolves `VERSION` from `VERSION` env, `GIT_TAG` env, or `git describe`
(defaults to `0.0.0`). Wraps `scripts/release.sh` so the same image can
produce any tag's artifacts.