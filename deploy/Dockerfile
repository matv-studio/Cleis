# Cleis organization server in a container (D-63).
#   docker build -f deploy/Dockerfile -t cleis-server .
#   docker run --rm -v cleis:/data cleis-server init --data /data --org "Example Studio"
#   docker run -d --name cleis -p 8443:8443 -v cleis:/data --restart unless-stopped cleis-server
FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p cleis-server

FROM debian:bookworm-slim
RUN useradd --system --home /data cleis && mkdir /data && chown cleis /data
COPY --from=build /src/target/release/cleis-server /usr/local/bin/cleis-server
USER cleis
ENV CLEIS_DATA=/data
VOLUME /data
EXPOSE 8443
HEALTHCHECK CMD ["cleis-server", "status", "--data", "/data"]
ENTRYPOINT ["cleis-server"]
CMD ["run", "--data", "/data"]
