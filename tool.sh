#!/usr/bin/env bash

docker="docker"
image="yoryan/simutrans-server"

"$docker" build --tag "$image:steam-standard" --file standard.dockerfile .

function build() {
    pak="$1"
    dlc_depot="$2"
    pak_dir="$3"
    if [[ -z "$pak_dir" ]]; then
        pak_dir="$pak"
    fi
    "$docker" build --tag "$image:steam-standard-$pak" \
        --build-arg "BASE_IMAGE=$image:steam-standard" \
        --build-arg "PAK_DIR=$pak_dir" \
        --build-arg "DLC_DEPOT=$dlc_depot" \
        --secret type=env,id=steam_login,env=STEAM_LOGIN \
        --file steam-pak.dockerfile .
}

case "$1" in
    'build')
        build   pak128          434529
        build   pak64           434630
        build   pak128.britain  434631  pak128.Britain
        build   pak192.comic    434632
        build   pak128.german   434633
        build   pak128.japan    435960
        build   pak64.german    435963

        exit 0
        ;;
    'push')
        paks=(
            pak64   pak64.german
            pak128  pak128.britain  pak128.german   pak128.japan
            pak192.comic
        )
        "$docker" push "$image:steam-standard"
        for pak in "${paks[@]}"; do
            "$docker" push "$image:steam-standard-$pak"
        done

        exit 0
        ;;
    *)
        exit 1
        ;;
esac