FROM ubuntu:22.04

# Don't ask questions during install
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates \
    python3 \
    python3-pip \
    python3-venv \
    nodejs \
    npm \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Set working directory
WORKDIR /workspace

# Copy everything
COPY . .

# Install Python dependencies (without venv)
WORKDIR /workspace/refact-server
RUN pip3 install --user -e .

# Install GUI dependencies
WORKDIR /workspace/refact-agent/gui
RUN npm install

# Build LSP binary
WORKDIR /workspace/refact-agent/engine
RUN cargo build --release

# Copy binary to accessible location
RUN cp target/release/refact-lsp /usr/local/bin/refact-lsp

# Go back to root
WORKDIR /workspace

# Expose ports
EXPOSE 8001 8002 8008

# Default command
CMD ["/bin/bash"]

