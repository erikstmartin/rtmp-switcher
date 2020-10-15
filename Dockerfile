FROM nvidia/cuda:11.1-devel-ubuntu20.04 AS build
RUN apt-get update

RUN echo 'debconf debconf/frontend select Noninteractive' | debconf-set-selections
RUN apt-get update && apt-get -y --no-install-recommends install \
    curl \
    openssl \
    libssl-dev \
    build-essential \
    autopoint \
    pkg-config \
    python3.8 \
    git \
    unzip \
    autoconf \
    automake \
    libtool \
    gstreamer-1.0 \
    gstreamer1.0-tools \
    libgstreamer-plugins-base1.0-dev \
    libnvidia-encode-455 \
    libnvidia-decode-455 \
    libfdk-aac-dev

RUN apt-get -y --no-install-recommends install vim

WORKDIR /build/gstreamer
COPY build_gst_plugins.sh ./

RUN ./build_gst_plugins.sh

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -sSf | \
      sh -s -- --default-toolchain stable -y
ENV PATH="/root/.cargo/bin:${PATH}"

# create a new empty shell project
RUN USER=root cargo new --bin /rtmp-switcher
WORKDIR /rtmp-switcher

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY src ./src
RUN cargo build --release

# TODO: Install dot (graphviz)
FROM nvidia/cuda:11.1-runtime-ubuntu20.04
RUN echo 'debconf debconf/frontend select Noninteractive' | debconf-set-selections
RUN apt-get update
RUN apt-get install -y libgstreamer1.0-0 gstreamer1.0-plugins-base libgstreamer-plugins-base1.0-dev gstreamer1.0-plugins-good \
gstreamer1.0-plugins-bad libgstreamer-plugins-bad1.0-0 gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-doc \ 
gstreamer1.0-tools gstreamer1.0-x fdkaac wget kmod
WORKDIR /build
# TODO: Is there another way to get libnvcuvid.so and libcuda.so 
RUN wget https://us.download.nvidia.com/XFree86/Linux-x86_64/455.28/NVIDIA-Linux-x86_64-455.28.run && \
  chmod +x ./NVIDIA-Linux-x86_64-455.28.run && \
  ./NVIDIA-Linux-x86_64-455.28.run -q -a -b --ui none --no-nvidia-modprobe --no-kernel-module
COPY --from=build /rtmp-switcher/target/release/switcher /usr/bin/rtmp-switcher
COPY --from=build /usr/local/lib/gstreamer-1.0 /usr/lib/x86_64-linux-gnu/gstreamer-1.0
CMD ["rtmp-switcher"]
