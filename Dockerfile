# FROM rust:1.45 as builder
# WORKDIR /usr/src/myapp
# COPY . .
# RUN apt show libc6
# RUN cargo build --release --target x86_64-unknown-linux-gnu

FROM debian:stretch-slim
WORKDIR /usr/src/myapp
RUN apt-get update
RUN apt-get install -y build-essential libssl-dev pkg-config libpq-dev wget
RUN wget https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init
RUN chmod +x rustup-init
RUN ./rustup-init -y
COPY . .
RUN /root/.cargo/bin/cargo build --release
# COPY --from=builder /usr/local/cargo/bin/myapp /usr/local/bin/myapp
# CMD ["myapp"]