<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# Getting started

## System requirements

Pullpiri has been tested with CentOS Stream 9.

[Bluechi](https://github.com/eclipse-bluechi/bluechi/tree/main) is required for Pullpiri.  
[Podman](https://podman.io/) needs to be installed as this is used as container runtime (Maybe podman is already installed in CentOS Stream 9).  
Also, [Rust](https://www.rust-lang.org) is required to test without using a container.

## Preliminary Info

### Pullpiri Configuration

There is a [settings.yaml](/src/settings.yaml) for configuration. Modify this to suit your system.

```yaml
yaml_storage: /etc/piccolo/yaml
piccolo_cloud: http://0.0.0.0:41234
host:
  name: HPC
  ip: 0.0.0.0
  type: bluechi
guest:
#  - name: ZONE
#    ip: 192.168.0.1
#    type: nodeagent
dds:
  idl_path: src/vehicle/dds/idl
  domain_id: 100
  # Removed out_dir - will use Cargo's default OUT_DIR
```

- yaml_storage : For making systemd service with podman, we need `.kube` and `.yaml` files.
- piccolo_cloud : The repository address saving `Packages` and `scenarios`.
- host : To deliver systemd command with `bluechi`, we need node name.
- guest : Bluechi agent node information.
- dds : will be updated.

### Pullpiri modules

Pullpiri consists of many modules.
For each modules, refer to [Structure](/doc/docs/developments.md#structure).  
And the [example](/examples/README.md) would be helpful.

## Limitations

- Multi-node system and the resulting node-selectors have not yet been fully considered.
- For better operation, recommend operating with `root` user with selinux permissive mode.
- `/etc/containers/systemd` folder is used for pullpiri systemd service files. This cannot be changed.
- Because it is still an early version, it may sometimes take a lot of time to start/stop/update the container.
- There may be other issues as well.

## Installation

### Before installation

need some packages, disable firewall, permissive selinux

```bash
# disable firewall
systemctl stop firewalld
systemctl disable firewalld
# install package
dnf install git-all make gcc -y
# permissive selinux
setenforce 0
```

For modifying configuration, see [configuration](#pullpiri-configuration).

### Install process

All Pullpiri applications with test app will start in container.
If you are familiar with container, you will find it easy to use.
`Pullpiri` also uses `podman play` by default.
If this is your first time, I recommend following [Example](/examples/README.md) first.

Before starting, you must build Pullpiri container image,

```sh
make builder
make image
```

*CAUTION* - A successful build requires at least 20GB of disk space.

If you have errors during `apt update`, then check dns nameserver.

For starting,

```sh
make install
```

For stoping,

```sh
make uninstall
```

You can see container list via `podman ps`. (infra container is omitted.)

```Text
[root@master pullpiri]# podman ps
CONTAINER ID  IMAGE                                 COMMAND               CREATED         STATUS         PORTS          NAMES
fd03b211e2ac  gcr.io/etcd-development/etcd:v3.5.11  --data-dir=/etcd-...  32 seconds ago  Up 32 seconds  2379-2380/tcp  piccolo-server-etcd
c6fbbb6feca5  localhost/pullpiri-server:latest                            32 seconds ago  Up 31 seconds                 piccolo-server-apiserver
341edada2c33  localhost/pullpiri-agent:latest                             31 seconds ago  Up 31 seconds                 piccolo-agent-nodeagent
eee2153bb581  localhost/pullpiri-player:latest                            31 seconds ago  Up 30 seconds                 piccolo-player-filtergateway
8d8011a24b43  localhost/pullpiri-player:latest                            31 seconds ago  Up 30 seconds                 piccolo-player-actioncontroller

[root@master images]# podman pod ps
POD ID        NAME            STATUS      CREATED             INFRA ID      # OF CONTAINERS
cc169812bd3e  piccolo-player  Degraded    About a minute ago  fb8974d9ba47  4
85eeff5e07cf  piccolo-agent   Running     About a minute ago  518c9482ae00  2
809508bfdc46  piccolo-server  Degraded    About a minute ago  1a738d6106f0  5
```

Also refer to [Makefile](/Makefile).
