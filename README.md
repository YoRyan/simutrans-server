# yoryan/simutrans-server

This repository contains Simutrans headless servers that are ready to run in Docker and are compatible with the version of the game available through [Steam](https://store.steampowered.com/app/434520/Simutrans/). Find auto-built images on [Docker Hub](https://hub.docker.com/r/yoryan/simutrans-server). Spin up a game for your fellow transportation nerds with ease!

## Tags

- `steam` (executable only; not playable)
- `steam-pak64`
- `steam-pak128`

## Usage

```
docker run -v C:\Users\Ryan\Documents\Simutrans\save\:/game/save yoryan/simutrans-server:steam-pak64 -load MyServerTemplateGame
```
