# first stage
FROM debian:stable AS builder

RUN dpkg --add-architecture i386 && apt update && apt install -y \
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

ARG REVISION=10333
RUN svn checkout svn://servers.simutrans.org@$REVISION
WORKDIR /code/simutrans/trunk
COPY ignore-pak-errors.patch ./
RUN patch -p0 <ignore-pak-errors.patch && \
  autoconf && ./configure && \
  make -j8 CC='gcc -m32 -static' CXX='g++ -m32 -static'
RUN ./get_lang_files.sh
RUN ./distribute.sh && \
  mkdir /code/dist && cd /code/dist && \
  unzip /code/simutrans/trunk/simulinux-${REVISION}M.zip
RUN groupadd -r simu && useradd --no-log-init -r -g simu simu

# second stage
FROM scratch
COPY --from=builder /etc/passwd /etc/group /etc/
COPY --from=builder --chown=simu:simu /code/dist/simutrans/ /game/
USER simu
ENTRYPOINT ["/game/simutrans", "-singleuser"]
EXPOSE 13353