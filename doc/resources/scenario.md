<!--
SPDX-License-Identifier: Apache-2.0
-->

# Scenario

A scenario is a specification for performing given actions on a given target package when given conditions are met.

The following is an example of a simple `scenario` from [launch-scenario.yaml](/examples/res/scenarios/launch-scenario.yaml).

```yaml
apiVersion: v1
kind: Scenario
metadata:
  name: version-display
spec:
  conditions:
    express: Equal
    value: "parking"
    operands:
      type: DDS
      name: gear_state
      value: "rt/pullpiri/gear_state"
  actions:
    - operation: launch
  targets:
    - name: "version-display-1"
```

## Condition

The conditions under which a vehicle can be used vary greatly. Currently, conditions are determined via DDS messages from the vehicle, with more conditions to be added in the future.

In the above example, the condition is met when the gear state is received by the DDS and the gear state is in park.

## Action

Actions are actions to be performed, such as download/update/launch/rollback/terminate.

## Target

A target is `package` resource name.
