#############
## Builder ##
#############

FROM --platform=$BUILDPLATFORM rust:slim AS builder

ARG TARGETARCH

# These are build platform depandant, but will be ignored if not needed
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="aarch64-linux-gnu-gcc"
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-C target-feature=+crt-static -C link-arg=-lgcc"
ENV CARGO_TARGET_ARM_UNKNOWN_LINUX_MUSLEABIHF_LINKER="arm-linux-gnueabihf-ld"

COPY ./containerised/target.sh .

RUN chmod +x ./target.sh && ./target.sh

RUN apt-get update && apt-get install $(cat /.compiler) -y

WORKDIR /usr/src

# Create workspace structure
RUN mkdir -p /usr/src/oxker

# Copy workspace configuration and crate manifests
COPY Cargo.toml /usr/src/oxker/
COPY oxker-core/Cargo.toml /usr/src/oxker/oxker-core/
COPY oxker-tui/Cargo.toml /usr/src/oxker/oxker-tui/

# Set the working directory
WORKDIR /usr/src/oxker

# Create dummy source files to cache dependencies
RUN mkdir -p oxker-core/src oxker-tui/src && \
    echo "fn main() {}" > oxker-tui/src/main.rs && \
    echo "pub fn dummy() {}" > oxker-core/src/lib.rs

# Install target platform (Cross-Compilation)
RUN rustup target add $(cat /.target)

# This is a dummy build to get the dependencies cached
RUN cargo build --target $(cat /.target) --release -p oxker-tui

# Now copy in the actual sources
COPY oxker-core/src /usr/src/oxker/oxker-core/src/
COPY oxker-tui/src /usr/src/oxker/oxker-tui/src/

# Touch main.rs to prevent cached release build
RUN touch /usr/src/oxker/oxker-tui/src/main.rs

# This is the actual application build
RUN cargo build --release --target $(cat /.target) -p oxker-tui

RUN cp /usr/src/oxker/target/$(cat /.target)/release/oxker /

#############
## Runtime ##
#############

FROM scratch

# Set an ENV to indicate that we're running in a container
ENV OXKER_RUNTIME=container

COPY --from=builder /oxker /app/

# Run the application
# this is used in the application itself so DO NOT EDIT
ENTRYPOINT [ "/app/oxker"]