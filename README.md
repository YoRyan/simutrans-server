# yoryan/simutrans-server

This repository contains Simutrans headless servers that are ready to run in Docker and are compatible with the version of the game available through [Steam](https://store.steampowered.com/app/434520/Simutrans/). Find auto-built images on [Docker Hub](https://hub.docker.com/r/yoryan/simutrans-server). Spin up a game for your fellow transportation nerds with ease!

This image contains a headless Simutrans executable that is compiled as 32-bit, which is preferred by the game. It also contains a wrapper program that makes Simutrans behave better as a service:

- Save data is copied to and from a dedicated `/save` directory, which can be easily stored inside a Docker volume or bind mount.
- Requests to stop the container are forwarded to Simutrans and handled gracefully with an autosave.
- The game is autosaved on a regular (real-world) time basis, by default every two hours. This is accomplished by periodically killing and restarting Simutrans.

## Tags

- `steam-standard` (executable only; not playable)
- `steam-standard-pak64`
- `steam-standard-pak64.german`
- `steam-standard-pak128`
- `steam-standard-pak128.britain`
- `steam-standard-pak128.german`
- `steam-standard-pak128.japan`
- `steam-standard-pak192.comic`

## Usage

For the first run, you can use `--populate` to copy an existing save file into the `/save` volume while bypassing any permissions problems:

```
docker run -it --rm -v ~/simutrans/save/mygame.sve:/mygame.sve:ro -v simutrans-server:/save yoryan/simutrans-server:steam-standard-pak64 --populate /mygame.sve
```

For regular operations, just run the container without any arguments:

```
docker run -v simutrans-server:/save yoryan/simutrans-server:steam-standard-pak64
```

A suggested Docker Compose service definition is as follows:

```yaml
simutrans:
  image: yoryan/simutrans-server:steam-standard-pak64
  restart: unless-stopped
  environment:
    RUST_LOG: info
  volumes:
    - ./simuconf.tab:/game/config/simuconf.tab:ro
    - simutrans:/save/
  ports:
    - "13353:13353"
```

If you choose to bind-mount `/save` instead of using a volume, make sure the mount is read-writable by the `999:999` user. The image is designed to run as this non-root user.

You can set simuconf.tab options by bind-mounting your own version of this file to `/game/config/simuconf.tab`. However, please ensure you have set `server_save_game_on_quit = 1` in this file; otherwise, the wrapper program will not be able to obtain autosaves from Simutrans.

You can pass command-line options directly to Simutrans by appending them to the container command:

```
docker run -v simutrans-server:/save yoryan/simutrans-server:steam-standard-pak64 -- -freeplay
```

For more information about the options accepted by the wrapper program, see:

```
docker run -it --rm yoryan/simutrans-server:steam-standard --help
```