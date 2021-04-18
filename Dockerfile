FROM rust
COPY . /root/project
WORKDIR /root/project
ENV API_Public_Key=vyThNuQQrOhYc0LlnddBbwt2Q8Tt3M2v3mW1hSO5eJ7FMNLowb1Pmm9y
ENV API_Private_Key=Ig6MZH7Aax7uqGSbV3I6McLVZcOx8MuQQNbDwZLI7qQMNQUZac/cemLCSkACEsISLEDBNZ7CJy8XVTUxl3sLyQ==
ENV OTP=CheradenineZakalwe1!
RUN cargo build --tests
CMD ["sh", "-c", "cargo test --tests"]
