# yoryan/simutrans-server

This repository contains Simutrans headless servers that are ready to run in Docker and are compatible with the version of the game available through [Steam](https://store.steampowered.com/app/434520/Simutrans/). Find auto-built images on [Docker Hub](https://hub.docker.com/r/yoryan/simutrans-server). Spin up a game for your fellow transportation nerds with ease!

Compared to compiling Simutrans yourself, this image has a few extra features:

- Runs as a 32-bit executable, which is preferred by Simutrans.
- Pak warning messages are patched out, which allows dated paks like pak128.japan to load.
- Network saves have been moved to the `save/` folder, allowing for easy management of save state in a Docker volume.

## Tags

- `steam` (executable only; not playable)
- `steam-pak64`
- `steam-pak128`
- `steam-pak128.german`

## Usage

Save state is stored in the `/game/save/` folder. On the first run, you can use the `-load` flag to load any save game in this folder.

```
docker run -v C:\Users\Ryan\Documents\Simutrans\save\:/game/save yoryan/simutrans-server:steam-pak64 -load MyServerTemplateGame
```

Simutrans will update the autosave file at `/game/save/server13353-network.sve` whenever a new client connects. You can omit the `-load` flag to load this autosave.

```
docker run -v C:\Users\Ryan\Documents\Simutrans\save\:/game/save yoryan/simutrans-server:steam-pak64
```
