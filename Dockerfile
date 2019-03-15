FROM liuchong/rustup:nightly AS build

RUN cargo install cargo-make

WORKDIR /build

COPY Cargo.* /build/
RUN mkdir -p src/bin && echo "" > src/lib.rs && echo "fn main() {}" > src/bin/evredis.rs && cargo build --release && rm -rf src

COPY . /build
RUN touch src/lib.rs && cargo make release-flow


FROM gcr.io/distroless/cc

COPY --from=build /build/target/package/release/bin /usr/bin
COPY --from=build /build/target/package/release/config /etc/xdg/evredis

ENV EVREDIS_LOGGING_LEVEL="debug"

ENTRYPOINT ["evredis"]
CMD ["server"]
