podman pod stop -t 0 piccolo-player
podman pod rm -f --ignore piccolo-player
podman pod stop -t 0 piccolo-server
podman pod rm -f --ignore piccolo-server