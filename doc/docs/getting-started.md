<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# Getting started

## System requirements

Piccolo has been tested with CentOS Stream 9.

[Bluechi](https://github.com/eclipse-bluechi/bluechi/tree/main) is required for Piccolo.  
[Podman](https://podman.io/) needs to be installed as this is used as container runtime (Maybe podman is already installed in CentOS Stream 9).  
Also, [Rust](https://www.rust-lang.org) is required to test without using a container.

## Preliminary Info

### Piccolo Configuration

There is a [settings.yaml](/src/settings.yaml) for configuration. Modify this to suit your system.

```yaml
yaml_storage: /root/piccolo_yaml
doc_registry: http://0.0.0.0:41234
host:
  name: HPC
  ip: 0.0.0.0
guest:
#  - name: ZONE
#    ip: 192.168.10.11
#    ssh_port: 22
#    id: root
#    pw: rootpassword
```

- yaml_storage : For making systemd service with podman, we need `.kube` and `.yaml` files. Lib `importer` makes these files in this directory.
- doc_registry : The repository address saving `Packages` and `scenarios`.
- host.name : To deliver systemd command with `bluechi`, we need node name.
- guest : Bluechi agent node information. ID/PW is required for `.kube`, `.yaml` files transfers.

### Piccolo modules

Piccolo consists of many modules.
For each modules, refer to [Structure](/doc/docs/developments.md#structure).  
~~And the [example](/examples/version-display/README.md) would be helpful.~~
Examples will be updated.

## Limitations

- Multi-node system and the resulting node-selectors have not yet been fully considered.
- For better operation, recommend operating with `root` user with selinux permissive mode.
- `/etc/containers/systemd` folder is used for piccolo systemd service files. This cannot be changed.
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

For modifying configuration, see [configuration](#piccolo-configuration).

### Install process

All Piccolo applications with test app will start in container.
If you are familiar with container, you will find it easy to use.
`Piccolo` also uses `podman play` by default.
If this is your first time, I recommend following [Example](/examples/version-display/README.md) first.

Before starting, you must build Piccolo container image,

```sh
make image
```

If you have errors during `apt update`, then check dns nameserver.

For starting,

```sh
make pre install
```

For stoping,

```sh
make uninstall post
```

You can see container list via `podman ps`.

```Text
[root@master piccolo-bluechi]# podman ps
CONTAINER ID  IMAGE                                    COMMAND               CREATED         STATUS         PORTS       NAMES
a89293d15b18  localhost/podman-pause:5.1.2-1720678294                        20 seconds ago  Up 21 seconds              a13f3aa439f3-service
ebce43e479be  localhost/podman-pause:5.1.2-1720678294                        20 seconds ago  Up 21 seconds              55f9dda92972-infra
53b9a1631df9  localhost/piccolo:1.0                                          20 seconds ago  Up 20 seconds              piccolo-apiserver
cd0683bb5675  localhost/piccolo:1.0                                          20 seconds ago  Up 21 seconds              piccolo-statemanager
eb8f60534077  gcr.io/etcd-development/etcd:v3.5.11     --data-dir=/etcd-...  20 seconds ago  Up 21 seconds              piccolo-etcd
9771320d5fee  localhost/piccolo:1.0                                          20 seconds ago  Up 21 seconds              piccolo-gateway

[root@master images]# podman pod ps
POD ID        NAME         STATUS      CREATED        INFRA ID      # OF CONTAINERS
55f9dda92972  piccolo      Running     6 minutes ago  ebce43e479be  5
```

Also refer to [Makefile](/Makefile).
