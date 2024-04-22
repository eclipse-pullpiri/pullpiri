# piccolo-bluechi
Piccolo for bluechi.

## Getting started
```sh
make build
```
Also refer to [Makefile](/Makefile).

## Using Ports
```
api-server : 47001
gateway : 47002
statemanager: 47003
yamlparser : 47004
etcd : 2379
```

## API
Refer to [BlueChi API document](https://github.com/eclipse-bluechi/bluechi/blob/main/doc/docs/api/description.md).

## BlueChi public D-Bus API

### interface org.eclipse.bluechi.Controller
| Bluechi        | Piccolo | Description                               |
| -------------- | :-----: | ----------------------------------------- |
| ListUnits      | X       | Returns all loaded systemd units          |
| CreateMonitor  | X       | Creates a new monitor object              |
| ListNodes      | O       | Returns information of all known nodes    |
| GetNode        | *       | Returns the path of a node given its name |
| EnableMetrics  | X       | Enables metrics on all connected agents   |
| DisableMetrics | X       | Disables metrics on all agents            |
| SetLogLevel    | X       | Set log level                             |

**Note** : (*) is used internally.

### interface org.eclipse.bluechi.Node
| Bluechi           | Piccolo | Description                         |
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
| SetLogLevel       | X       | Set log level for bluechi-agent     |

Set the new log level for bluechi-agent by invoking the internal bluechi-agent API.

### interface org.eclipse.bluechi.Monitor
Currently not supported.
### interface org.eclipse.bluechi.Job
Currently not supported.

## BlueChi-Agent public D-Bus API
### interface org.eclipse.bluechi.Agent
Currently not supported.
### interface org.eclipse.bluechi.Metrics
Currently not supported.

## Internal D-Bus APIs
### interface org.eclipse.bluechi.internal.Controller
Currently not supported.
### interface org.eclipse.bluechi.internal.Agent
Currently not supported.
### interface org.eclipse.bluechi.internal.Proxy
Currently not supported.
### interface org.eclipse.bluechi.internal.Agent.Metrics
Currently not supported.

## License
The LICENSES directory contains all the licenses used by the PICCOLO Project.