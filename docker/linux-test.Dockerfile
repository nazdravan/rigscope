FROM rust:1.95-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends \
        xvfb xauth libxkbcommon-x11-0 libxkbcommon0 libgl1 libegl1 libgl1-mesa-dri \
        libx11-xcb1 libxcursor1 libxrandr2 libxi6 libwayland-client0 libgtk-3-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN rustup component add clippy rustfmt
ENV CARGO_TARGET_DIR=/target
