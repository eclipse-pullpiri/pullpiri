<!--
SPDX-License-Identifier: Apache-2.0
-->

# Package

A piccolo package exposes container image information, network, volume requirements, etc. By separating these resources, Piccolo aims to make it easier for users to assemble the services they want.

The following is an example of a simple `package` from [version-display-1.tar](/examples/res/packages/version-cli-1.tar). The package is used in tar format, but for convenience, the extracted folder is placed in the [path](/examples/res/packages/version-display-1/).

```Text
[root@master version-display-1]# tree
.
├── models
│   └── version-display.yaml
├── networks
│   └── vd-network.yaml
├── package.yaml
└── volumes
    └── vd-volume.yaml
```

## package.yaml

Every package has 1 `package.yaml` file. This file contains `model`, abstract system resource like `volume` and `network`. More fields can be added.

By combining container information and vehicle resources within the `model`, a package can be created, allowing vehicle services to be implemented in various combinations.

```yaml
apiVersion: v1
kind: Package
metadata:
  label: null
  name: version-display-1
spec:
  pattern:
    - type: plain
  models:
    - name: version-display     # model name
      resources:
        volume: vd-volume       # volume name
        network: vd-network     # network name
```

## Model

A `model` is similar to Pod in Kubernetes.

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: version-display
  labels:
    app: version-display
spec:
  containers:
    - name: display-container
      image: localhost/version-display:1.0
      volumeMounts:
        - name: x11
          mountPath: /tmp/.X11-unix
      env:
        - name: DISPLAY
          value: :0
  terminationGracePeriodSeconds: 0
```

## Network

Many vehicle services have different network requirements, and it is difficult to add them one by one when writing container specifications. Therefore, various network information is abstracted into different resources, and packages combine them to easily create services.

```yaml
apiVersion: v1
kind: Network
metadata:
  label: null
  name: vd-network
spec:
  dummy: network123     # not implemented yet

```

## Volume

As with `network`, the goal is to provide resources for each volume of information.

```yaml
apiVersion: v1
kind: Volume
metadata:
  label: null
  name: vd-volume
spec:
  volumes:
    - name: x11
      hostPath:
        path: /tmp/.X11-unix
```
