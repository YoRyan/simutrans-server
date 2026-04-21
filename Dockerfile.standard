# Build simutrans
FROM debian:trixie AS simutrans

RUN dpkg --add-architecture i386 && apt-get update && apt-get install -y \
  autoconf \
  build-essential \
  curl \
  g++-multilib \
  libbz2-dev:i386 \
  libminiupnpc-dev:i386 \
  libpng-dev:i386 \
  libzstd-dev:i386 \
  subversion \
  zip \
  zlib1g-dev:i386
WORKDIR /code

ARG REVISION=11919
RUN svn checkout svn://servers.simutrans.org@$REVISION
WORKDIR /code/simutrans/trunk
RUN autoconf && ./configure && \
  make -j32 CC='gcc -m32 -static' CXX='g++ -m32 -static'
RUN ./tools/distribute.sh && \
  mkdir /code/dist && cd /code/dist && \
  unzip /code/simutrans/trunk/simulinux-${REVISION}.zip

# Build entrypoint
FROM rust:1-trixie AS entrypoint

COPY entrypoint /code
WORKDIR /code
RUN cargo build --release

# Final image
FROM debian:trixie
RUN groupadd -r simu && useradd --no-log-init -r -g simu simu
COPY --from=simutrans --chown=simu:simu /code/dist/simutrans/ /game/
COPY --from=entrypoint /code/target/release/entrypoint /entrypoint
USER simu
ENTRYPOINT /entrypoint
VOLUME /save
EXPOSE 13353