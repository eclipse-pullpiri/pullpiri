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
[Download link](https://github.com/etcd-io/etcd/releases/download/v3.5.21/etcd-v3.5.21-linux-arm64.tar.gz)
: untar `tar.gz` file, you can see `etcdctl` binary.
If you put this binary in a PATH like `/usr/bin`, you can use it without installing anything.

*Optional* :
CentOS stream (or RHEL) + Eclipse Bluechi (Maybe you will need it later)

## Make container image

In root folder (maybe `pullpiri`), run

```bash
make builder
make image
```

And then you can check

```text
podman imagse

REPOSITORY                      TAG         IMAGE ID      CREATED             SIZE
localhost/pullpiri-server       latest      c075222701fd  About a minute ago  22.5 MB
<none>                          <none>      7d10b78d6046  About a minute ago  1.4 GB
localhost/pullpiri-player       latest      65dec5e51782  2 minutes ago       17.7 MB
<none>                          <none>      00745f9dcc27  2 minutes ago       1.47 GB
localhost/pullpiri-observer     latest      10621675f045  4 minutes ago       15.6 MB
<none>                          <none>      ff941faf947e  4 minutes ago       1.38 GB
localhost/pullpirirelease       latest      6fef0c340631  7 minutes ago       15.2 MB
<none>                          <none>      488de000d246  7 minutes ago       874 MB
localhost/pullpiribuilder       latest      5a7c4a8edfa8  7 minutes ago       886 MB
```

## Run piccolo

Piccolo consists of many modules, so I am only going to talk about how to run
the `apiserver` (which is almost complete). The rest will be updated later.

### Setting file

In root folder, you can find `settings.yaml`. Currently, there is no special correction.

```yaml
yaml_storage: /etc/piccolo
piccolo_cloud: http://0.0.0.0:41234
host:
  name: HPC
  ip: 0.0.0.0
guest:
#  - name: ZONE
#    ip: 192.168.0.1
```

### Start etcd container

etcd is database for piccolo.

```bash
podman run -it -d --net=host --name=piccolo-etcd \
gcr.io/etcd-development/etcd:v3.5.11 "/usr/local/bin/etcd"
```

### Start apiserver container

```bash
podman run -it -d --net=host \
-v ./src/settings.yaml:/piccolo/settings.yaml \
--name=piccolo-apiserver  \
localhost/pullpiri-server:latest "/piccolo/apiserver"
```

### Check two containers

```md
# run "podman ps"
podman ps
CONTAINER ID  IMAGE                                 COMMAND               CREATED         STATUS         PORTS          NAMES
c3dfb8ed511f  gcr.io/etcd-development/etcd:v3.5.11  /usr/local/bin/et...  23 minutes ago  Up 23 minutes  2379-2380/tcp  piccolo-etcd
0421b70cfe13  localhost/pullpiri-server:latest      /piccolo/apiserve...  22 minutes ago  Up 22 minutes                 piccolo-apiserver

# run "podman logs -f piccolo-apiserver"
podman logs -f piccolo-apiserver
http api listening on 0.0.0.0:47099
```

### Put Piccolo scenario into apiserver by shell script

In this folder (`examples`), there is a `helloworld.sh` script.
When you run this, you call the PUT method with the apiserver's HTTP API.

```sh
./helloworld.sh
```

### Check logs

If you look at the apiserver log after running the script, you will get
an error, which is caused by the lack of another module (filtergateway),
so you don't have to worry about it.

```md
# run "podman logs -f piccolo-apiserver"
[root@HPC examples]# podman logs -f piccolo-apiserver 
http api listening on 0.0.0.0:47099

thread 'tokio-runtime-worker' panicked at apiserver/src/grpc/sender/filtergateway.rs:26:10:
called `Result::unwrap()` on an `Err` value: tonic::transport::Error(Transport, ConnectError(ConnectError("tcp connect error", Os { code: 111, kind: ConnectionRefused, message: "Connection refused" })))
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

However, you can check if the parsing is done well through `etcdctl`.

```text
[root@HPC examples]# etcdctl get --prefix=true ""
Model/helloworld-core                             ---> KEY
apiVersion: v1                                    ---> VALUE
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
  - name: helloworld
    image: helloworld
  terminationGracePeriodSeconds: 0

Package/helloworld                                ---> KEY
apiVersion: v1                                    ---> VALUE
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
  - type: plain
  models:
  - name: helloworld-core
    node: HPC
    resources:
      volume: null
      network: null

Scenario/helloworld                               ---> KEY
apiVersion: v1                                    ---> VALUE
kind: Scenario
metadata:
  name: helloworld
spec:
  condition: null
  action: update
  target: helloworld
```

### Delete containers after testing

```sh
podman rm -f piccolo-etcd 
podman rm -f piccolo-apiserver 
```
