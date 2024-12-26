<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# Supported Bluechi APIs

Refer to [BlueChi API document](https://github.com/eclipse-bluechi/bluechi/blob/main/doc/docs/api/description.md).

## Public D-Bus API

### Controller

| Command        | Piccolo | Description                               |
| -------------- | :-----: | ----------------------------------------- |
| ListUnits      | X       | Returns all loaded systemd units          |
| CreateMonitor  | X       | Creates a new monitor object              |
| ListNodes      | O       | Returns information of all known nodes    |
| GetNode        | *       | Returns the path of a node given its name |
| EnableMetrics  | X       | Enables metrics on all connected agents   |
| DisableMetrics | X       | Disables metrics on all agents            |
| SetLogLevel    | X       | Set log level                             |

**Note** : (*) is used internally.

### Node

| Command           | Piccolo | Description                         |
| ----------------- | :-----: | ----------------------------------- |
| StartUnit         | O       | Start named unit                    |
| StopUnit          | O       | Stop named unit                     |
| ReloadUnit        | O       | Reload named unit                   |
| RestartUnit       | O       | Restart named unit                  |
| EnableUnitFiles   | O       | Enable one (or more) unit file      |
| DisableUnitFiles  | O       | Disable one (or more) unit file     |
| GetUnitProperties | X       | Returns properties for a named unit |
| GetUnitProperty   | X       | Get one named property              |
| SetUnitProperties | X       | Set named properties                |
| ListUnits         | O       | Returns all loaded units on node    |
| Reload            | O       | Reload all unit files               |
| SetLogLevel       | X       | Set log level for node agent        |

## the others

Not reviewed yet
