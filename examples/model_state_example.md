# Model State Management Example

This document demonstrates how the StateManager Model state management works based on container states.

## Overview

The StateManager automatically determines Model states based on the states of associated containers, following the specification in `doc/architecture/KR/3.LLD/StateManager_Model.md`.

## Container-Model Association

Containers are associated with Models using the `pullpiri.model` annotation in the container metadata:

```yaml
container_annotation:
  pullpiri.model: "my-inference-model"
```

## State Determination Logic

The StateManager implements the following state determination rules:

| Container Condition | Model State (Doc) | Model State (Proto) | Description |
|-------------------|------------------|-------------------|-------------|
| No containers | Created | Pending | Model has been created but no containers exist |
| All containers paused | Paused | Unknown | All containers are in paused state |
| All containers exited | Exited | Succeeded | All containers have completed execution |
| Any container dead | Dead | Failed | One or more containers are in dead/error state |
| Otherwise | Running | Running | Default running state |

## Example Scenarios

### Scenario 1: Model with Running Containers
```
Container 1: Status=running, Running=true, Paused=false, Dead=false
Container 2: Status=running, Running=true, Paused=false, Dead=false
Result: Model State = Running
```

### Scenario 2: Model with All Paused Containers
```
Container 1: Status=paused, Running=false, Paused=true, Dead=false
Container 2: Status=paused, Running=false, Paused=true, Dead=false
Result: Model State = Unknown (Paused)
```

### Scenario 3: Model with Dead Container
```
Container 1: Status=running, Running=true, Paused=false, Dead=false
Container 2: Status=dead, Running=false, Paused=false, Dead=true
Result: Model State = Failed (Dead)
```

### Scenario 4: Model with All Exited Containers
```
Container 1: Status=exited, Running=false, Paused=false, Dead=false
Container 2: Status=exited, Running=false, Paused=false, Dead=false
Result: Model State = Succeeded (Exited)
```

## ETCD Storage Format

Model states are stored in etcd with the following format:

```
Key: /model/{model_name}/state
Value: MODEL_STATE_RUNNING | MODEL_STATE_SUCCEEDED | MODEL_STATE_FAILED | MODEL_STATE_UNKNOWN | MODEL_STATE_PENDING
```

## Processing Flow

1. **Container Update Reception**: StateManager receives ContainerList from NodeAgent
2. **Container Grouping**: Containers are grouped by their `pullpiri.model` annotation
3. **State Determination**: For each model, determine state based on container states
4. **ETCD Storage**: Save the determined model state to etcd
5. **Logging**: Comprehensive logging for monitoring and debugging

## Log Output Example

```
=== PROCESSING CONTAINER LIST ===
  Node Name: worker-node-01
  Container Count: 3
  Container 1: inference-model-container-1
    Image: my-org/inference-model:v1.0
    State: {"Status": "running", "Running": "true", "Paused": "false", "Dead": "false"}
    ID: abc123
    Annotations: {"pullpiri.model": "my-inference-model"}
    Associated with Model: my-inference-model
--- Processing Model: my-inference-model ---
  Associated Containers: 2
    Container abc123 status: running (running: true, paused: false, dead: false)
    Container def456 status: running (running: true, paused: false, dead: false)
  → Model state: Running
  Determined Model State: Running
    Saving to etcd - Key: /model/my-inference-model/state, Value: MODEL_STATE_RUNNING
    Successfully saved model state to etcd
  Model state saved to etcd successfully
  Status: Container list processing completed
```

This implementation provides automatic model state management based on container health, enabling proper monitoring and orchestration of model workloads in the PICCOLO framework.