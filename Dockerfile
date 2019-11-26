FROM rust:1.39-slim-buster as build
ADD ./ /app
WORKDIR /app
RUN cargo clean
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release

FROM debian:buster-slim as rt
RUN groupadd -g 999 appuser && \
    useradd -r -u 999 -g appuser appuser
USER appuser
WORKDIR /app
COPY --from=build /app/target/release/actix-bb8-try .
CMD [ "./actix-bb8-try" ]