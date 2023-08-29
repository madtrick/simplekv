FROM rust:1.70
WORKDIR /usr/src/myapp
COPY . .
RUN cargo install --path .
ENTRYPOINT ["kv", "--port", "1339", "--id", "2"]
