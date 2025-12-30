FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

ARG RUST_TOOLCHAIN="1.91.0"
ARG ANCHOR_CLI="0.32.1"
ARG SOLANA_CLI="2.2.0"
ARG NVM_VERSION="0.40.2"
ARG NODE_VERSION="node"

RUN apt-get update && \
    apt-get -y install libssl-dev libudev-dev pkg-config zlib1g-dev llvm clang cmake make libprotobuf-dev libsasl2-dev && \
    rm -rf /var/lib/apt/lists/*

USER vscode
WORKDIR /home/vscode

SHELL ["/bin/zsh", "-c"]
ENV NVM_DIR /home/vscode/.nvm
ENV PATH="/home/vscode/.cargo/bin:$PATH"

RUN mkdir $NVM_DIR

RUN curl -o- https://raw.githubusercontent.com/creationix/nvm/v${NVM_VERSION}/install.sh | bash
RUN source $NVM_DIR/nvm.sh && \
    nvm install ${NODE_VERSION} && \
    nvm use ${NODE_VERSION} && \
    nvm alias default node && \
    echo 'export NVM_DIR="$HOME/.nvm"' >> ~/.zshrc && \
    echo '[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"' >> ~/.zshrc && \
    echo 'export PATH="/home/vscode/.local/bin:$PATH"' >> ~/.zshrc

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    rustup toolchain install ${RUST_TOOLCHAIN} && \
    rustup default ${RUST_TOOLCHAIN} 

RUN sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
RUN cargo install --git https://github.com/solana-foundation/anchor.git anchor-cli --locked --version ${ANCHOR_CLI}

RUN echo 'PATH="/home/vscode/.local/bin:$PATH"' >> ~/.zshrc
