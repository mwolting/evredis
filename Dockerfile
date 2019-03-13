FROM liuchong/rustup:nightly AS build

RUN cargo install cargo-make

WORKDIR /build

COPY . /build
RUN cargo make release-flow


FROM gcr.io/distroless/cc

COPY --from=build /build/target/package/release/bin /usr/bin
COPY --from=build /build/target/package/release/config /etc/xdg/axon

ENV EVREDIS_LOGGING_LEVEL="info"

ENTRYPOINT ["evredis"]
CMD ["server"]
