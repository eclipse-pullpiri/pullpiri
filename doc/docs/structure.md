<!--
SPDX-License-Identifier: Apache-2.0
-->

# Structure

<img alt="piccolo overview" src="../images/overview.png"
width="75%"
height="75%"
/>

```bash
.
├── containers      # files for binary container
├── doc
│   ├── docs
│   └── images
├── examples
│   ├── nginx       # (optional) file server
│   └── resources   # package and scenario files
├── LICENSES
└── src
    ├── api         # gRPC proto files
    ├── apiserver
    ├── common      # common library
    ├── gateway
    ├── importer
    ├── statemanager
    └── vendor
        └── lge_dds # DDS receiver for gateway
```

## apiserver

Apiserver works similarly to apiserver in k8s.
It parses resources received via REST API through `importer` and writes actions and conditions to `etcd` so that the `gateway` can recognize the conditions.
In addition, it can access `statemanager` directly.

We are also developing to collect metrics for monitoring.

## importer

Importer is responsible for parsing resource files into the necessary items for piccolo.

Specifically, create a `.kube` file and a `.yaml` file in the PICCOLO_YAML path, separate the scenario and package into condition and action and target, and pass it to the apiserver.

## gateway

The gateway receives a vehicle message according to the condition received from the apiserver and notifies the statemanager when the condition is satisfied.
It is currently written in C++, but a version rewritten in Rust is nearly complete and will soon be replaced.

## statemanager

The statemanager calls the other workload orchestrator API based on a message from the gateway or apiserver.
Therefore, it is the destination that must be passed through in order to execute the workload.
Specifically, when it receives a notification from the gateway that the condition has been satisfied, it pulls out the corresponding action from etcd and executes it.

In addition to the API calls, the complexity increases for reconcile tasks, etc., so it will be separated into several modules.

## etcd

The etcd stores data that is commonly used by each Piccolo module.
Writes are made only from the apiserver, and the gateway and statemanager read them to perform the necessary actions.

## others (will be updated)

- metric-notifier : We will provide a dashboard to monitor the status of each package and scenario.
- tools - dds-idl-gen : Vehicle messages sent via DDS use the IDL file format. This is a tool that converts IDL into a struct used in Rust modules.
- tools - workloadconverter : A tool to convert resources used in other orchestrators such as k8s to Piccolo resources. However, it is currently deprecated.

<!-- markdownlint-disable-file MD033 no-inline-html -->