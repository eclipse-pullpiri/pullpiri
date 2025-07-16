# Run Pullpiri (a.k.a. Piccolo) examples

This document is for LG's internal reference and will be updated separately
under the `doc` folder for external (official) use.

## Preparation

Basically, there is Piccolo-related documents in `doc/docs` folder, but there
are many parts that are different from the present as past data, so it is
only used for reference.

You need to install `Podman` with container runtime.
You can also use `Docker` (but **NOT** recommend).

It is STRONGLY recommended to install
[`etcdctl`](https://github.com/etcd-io/etcd/blob/main/etcdctl/README.md)
for low-level verification.
[Download link](https://github.com/etcd-io/etcd/releases)

Download appropriate `tar.gz` and untar file, you can see `etcdctl` binary.
If you put this binary in a PATH like `/usr/bin`, you can use it without installing anything.

*Optional* :
CentOS stream (or RHEL) + Eclipse Bluechi (Maybe you will need it later)

## Make container image & Run Piccolo

Refer [Getting started](/doc/docs/getting-started.md) for launching Piccolo.
All you have to do is run `make install` and you're ready to go.

### Check logs of containers

```md
# [root@HPC Pullpiri]# podman logs piccolo-server-apiserver 
http api listening on 0.0.0.0:47099
# [root@HPC Pullpiri]# podman logs piccolo-player-filtergateway 
FilterGatewayManager init
Piccolod gateway listening on 0.0.0.0:47002
FilterGatewayManager successfully initialized
# [root@HPC Pullpiri]# podman logs piccolo-player-actioncontroller 
Starting ActionController...
Starting gRPC server on 0.0.0.0:47001
gRPC server started and listening
```

## Run example

### Put Piccolo scenario into apiserver by shell script

In this folder (`examples`), there is a `helloworld.sh` script.
Replace `NODE_NAME` in `spec:models[0]:node` with your bluechi controller node name. (My node is `HPC`)

```yaml
......
---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld
      node: {NODE_NAME}
      resources:
        volume:
        network:
---
......
```

When you run this, you call the PUT method with the apiserver's HTTP API.

```sh
./helloworld.sh
```

### Check logs

You can see `helloworld` container (is restart infinitly.)

```md
# podman ps
39ab0e9e945f  quay.io/podman/hello:latest           /usr/local/bin/po...  6 seconds ago   Exited (0) Less than a second ago                 helloworld-helloworld
# podman logs helloworld-helloworld
!... Hello Podman World ...!

         .--"--.           
       / -     - \         
      / (O)   (O) \        
   ~~~| -=(,Y,)=- |         
    .---. /`  \   |~~      
 ~/  o  o \~~~~.----. ~~   
  | =(X)= |~  / (O (O) \   
   ~~~~~~~  ~| =(Y_)=-  |   
  ~~~~    ~~~|   U      |~~ 

Project:   https://github.com/containers/podman
Website:   https://podman.io
Desktop:   https://podman-desktop.io
Documents: https://docs.podman.io
YouTube:   https://youtube.com/@Podman
X/Twitter: @Podman_io
Mastodon:  @Podman_io@fosstodon.org
```

If you want to see more information, use `podman logs` for other containers.

### Clear

In root foler,

```sh
make uninstall
podman rm -f helloworld-helloworld
podman pod rm -f helloworld
```
