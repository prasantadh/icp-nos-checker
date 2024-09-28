FROM rust:1.81 AS base
WORKDIR /code
COPY . /code
RUN cargo build --release

FROM rust:1.81 AS production
EXPOSE 8080
COPY --from=base /code/target/release/single /checker
# CMD ["/bin/sleep", "infinity"]
CMD ["/checker"]
