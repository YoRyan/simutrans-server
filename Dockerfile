# first stage
FROM debian:stable AS builder

RUN dpkg --add-architecture i386 && apt update && apt install -y \
  build-essential \
  curl \
  g++-multilib \
  libbz2-dev:i386 \
  libminiupnpc-dev:i386 \
  libpng-dev:i386 \
  subversion \
  zip \
  zlib1g-dev:i386 
RUN groupadd -r simu && useradd --no-log-init -r -g simu simu
USER simu
WORKDIR /code

ARG REVISION=10333
RUN svn checkout svn://servers.simutrans.org@$REVISION
WORKDIR /code/simutrans/trunk
COPY config.default ignore-pak-errors.patch ./
RUN patch -p0 <ignore-pak-errors.patch && \
  echo "#define REVISION $REVISION" >revision.h && \
  make -j8 CFLAGS='-m32 -static' LDFLAGS='-m32 -static'
RUN ./get_lang_files.sh
RUN ./distribute.sh && \
  mkdir /code/dist && cd /code/dist && \
  unzip /code/simutrans/trunk/simulinux-${REVISION}M.zip

# second stage
FROM scratch
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /code/dist/simutrans/ /game/
USER simu
ENTRYPOINT ["/game/simutrans", "-singleuser"]
EXPOSE 13353