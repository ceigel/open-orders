FROM rust
COPY . /root/project
WORKDIR /root/project
RUN cargo build --tests
CMD cargo test --tests
