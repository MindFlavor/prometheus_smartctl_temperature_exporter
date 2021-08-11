ARG BUILDPLATFORM=linux/amd64

ARG ALPINE_VERSION=3.12
ARG RUST_VERSION=1-slim-bullseye

FROM --platform=${BUILDPLATFORM} rust:${RUST_VERSION} AS base
WORKDIR /usr/src/prometheus_smartctl_temperature_exporter

# Setup
RUN apt-get update -y && \
    apt-get install -y \
    # to cross build with musl
    musl-tools \
    # to download the musl cross build tool
    wget \
    # for verifying the binary properties
    file

# Download dependencies
RUN mkdir src && \
    echo 'fn main() {}' > src/main.rs
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch && \
    rm src/main.rs

ARG STATIC=yes
RUN touch /tmp/rustflags && \
    if [ "${STATIC}" != "yes" ]; then \
      echo "-C target-feature=-crt-static" | tee /tmp/rustflags; \
    fi

ARG TARGETPLATFORM
RUN echo "Setting variables for ${TARGETPLATFORM:=linux/amd64}" && \
    case "${TARGETPLATFORM}" in \
      linux/amd64) \
        MUSL="x86_64-linux-musl"; \
        RUSTTARGET="x86_64-unknown-linux-musl"; \
        break;; \
      linux/arm64) \
        MUSL="aarch64-linux-musl"; \
        RUSTTARGET="aarch64-unknown-linux-musl"; \
        break;; \
      linux/arm/v7) \
        MUSL="armv7m-linux-musleabi"; \
        RUSTTARGET="armv7-unknown-linux-musleabi"; \
        break;; \
      linux/arm/v6) \
        MUSL="armv6-linux-musleabi"; \
        RUSTTARGET="arm-unknown-linux-musleabi"; \
        break;; \
      linux/386) \
        MUSL="i686-linux-musl"; \
        RUSTTARGET="i686-unknown-linux-musl"; \
        break;; \
      linux/ppc64le) \
        MUSL="powerpc64le-linux-musl"; \
        RUSTTARGET="powerpc64le-unknown-linux-musl"; \
        break;; \
      linux/s390x) \
        MUSL="s390x-linux-musl"; \
        RUSTTARGET="s390x-unknown-linux-musl"; \
        break;; \
      linux/riscv64) \
        MUSL="riscv64-linux-musl"; \
        RUSTTARGET="riscv64gc-unknown-linux-musl"; \
        break;; \
      *) echo "unsupported platform ${TARGETPLATFORM}"; exit 1;; \
    esac && \
    echo "${MUSL}" | tee /tmp/musl && \
    echo "${RUSTTARGET}" | tee /tmp/rusttarget

RUN MUSL="$(cat /tmp/musl)" && \
    wget -qO- "https://musl.cc/$MUSL-cross.tgz" | tar -xzC /tmp && \
    rm "/tmp/$MUSL-cross/usr" && \
    cp -fr /tmp/"$MUSL"-cross/* / && \
    rm -rf "/tmp/$MUSL-cross"

RUN rustup target add "$(cat /tmp/rusttarget)"

# Copy .cargo/config for cross build configuration
COPY .cargo ./.cargo

# Install Clippy for build platform
RUN rustup component add clippy

# Install dependencies
RUN echo 'fn main() {}' > src/main.rs && \
    RUSTFLAGS="$(cat /tmp/rustflags)" \
    CC="$(cat /tmp/musl)-gcc" \
    cargo build --target "$(cat /tmp/rusttarget)" --release
RUN rm -r \
    target/*-linux-*/release/deps/prometheus_smartctl_temperature_exporter* \
    target/*-linux-*/release/prometheus_smartctl_temperature_exporter* \
    src/main.rs

COPY . .

FROM base AS lint
RUN RUSTFLAGS="$(cat /tmp/rustflags)" \
    CC="$(cat /tmp/musl)-gcc" \
    cargo clippy --target "$(cat /tmp/rusttarget)"

FROM base AS test
ENTRYPOINT \
    RUSTFLAGS="$(cat /tmp/rustflags)" \
    CC="$(cat /tmp/musl)-gcc" \
    cargo test --target "$(cat /tmp/rusttarget)"

FROM base AS build
# Build static binary with musl built-in
RUN RUSTFLAGS="$(cat /tmp/rustflags)" \
    CC="$(cat /tmp/musl)-gcc" \
    cargo build --target "$(cat /tmp/rusttarget)" --release && \
    mv target/*-linux-*/release/prometheus_smartctl_temperature_exporter /tmp/binary
RUN description="$(file /tmp/binary)" && \
    echo "$description" && \
    if [ "${STATIC}" = "yes" ] && [ ! -z "$(echo $description | grep musl)" ]; then \
      echo "binary is not statically built!" && exit 1; \
    fi

FROM alpine:${ALPINE_VERSION}
EXPOSE 9586/tcp
WORKDIR /usr/local/bin
RUN apk add --no-cache --q tini 
RUN apk add --no-cache --q smartmontools
RUN apk add --no-cache --q lsblk
RUN rm -rf /var/cache/apk/*

RUN adduser prometheus-smartctl-temperature-exporter -s /bin/sh -D -u 1000 1000 && \
    mkdir -p /etc/sudoers.d && \
    echo 'prometheus-smartctl-temperature-exporter ALL=(root) NOPASSWD:/usr/sbin/smartctl -a -j *' > /etc/sudoers.d/prometheus-smartctl-temperature-exporter && \
    chmod 0440 /etc/sudoers.d/prometheus-smartctl-temperature-exporter
RUN apk add --update -q --no-cache wireguard-tools-wg sudo
USER prometheus-smartctl-temperature-exporter
ENTRYPOINT ["/sbin/tini", "--", "/usr/local/bin/prometheus_smartctl_temperature_exporter"]
CMD [ "-a", "-v" ]
COPY --from=build --chown=prometheus-smartctl-temperature-exporter /tmp/binary ./prometheus_smartctl_temperature_exporter
