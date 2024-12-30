<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# Piccolo API (in progress)

TBD

## about scenarios

### Deploy new scenario

```plaintext
POST /scenario
```

#### Parameters

None

#### Request body

{scenario_name}/{scenario_file_name}

Example

```text
bms/bms-high-performance
```

#### Response

| Code  | Description |
| ------| -----       |
| 200   | Success     |
| 404   | Fail        |

```json
# Success
{
    "resp" : "Ok"
}

# Fail
{
    "resp" : "Error message"
}
```

### Delete scenario

```plaintext
DELETE /scenario/{scenario_name}
```

#### Parameters

scenario name you want to delete

#### Request body

None

#### Response

| Code  | Description |
| ------| -----       |
| 200   | Success     |
| 404   | Fail        |

```json
# Success
{
    "resp" : "Ok"
}

# Fail
{
    "resp" : "Error message"
}
```

## Metric

### Get container information

```plaintext
GET /metric/container
```

#### Parameters

None

#### Request body

None

#### Response

| Code  | Description |
| ------| -----       |
| 200   | Success     |
| 404   | Fail        |

JSON

```json
{
    "containers" : [
        {
            "id": "5bf8af556e997e007f068eb468e20b6ef8c2449dcbcaffdc1189d5",
            "names": [
                "bms-frism-frism"
            ],
            "image": "localhost/frism:1.0",
            "state": {
                "StartedAt": "2024-10-10T20:55:12.124155853+09:00",
                "ExitCode": "0",
                "...": "...",
                "Restart ing": "false",
                "OOMKilled": "false"
            },
            "config": {
                "AttachStdout": "false",
                "Image": "localhost/frism:1.0",
                "...": "...",
                "Hostname": "ZONE"
            },
            "annotation": {
                "io.kubernetes.cri-o.SandboxID": "d375a39c129874b8a3630a6",
****            "io.piccolo.annotations.package-network": "default",
****            "io.piccolo.annotations.package-type": "default",
                "org.opencontainers.image.stopSignal": "15",
****            "io.piccolo.annotations.package-name": "bms",
                "io.container.manager": "libpod"
            }
        },
        {
            "....": "....."
        }
    ]
}
```

### Get scenario information

```text
GET /metric/scenario
```

#### Parameters

None

#### Request body

None

#### Response

| Code  | Description |
| ------| -----       |
| 200   | Success     |
| 404   | Fail        |

JSON

```json
[
    {
        "name": "scneario name",
        "status": "active",
        "condition": "speed lt 30",
        "action": "close window"
    },
    {
        "name": "scneario name 2",
        "status": "inactive",
        "condition": "night",
        "action": "turn on light"
    }
]
```

<!-- markdownlint-disable-file MD024 no-duplicate-heading -->