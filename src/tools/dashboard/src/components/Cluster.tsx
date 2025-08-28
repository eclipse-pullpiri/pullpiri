import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "./ui/table";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Search, MoreHorizontal, Plus, Server, Cpu, MemoryStick, HardDrive, Activity, AlertTriangle, CheckCircle } from "lucide-react";

export function Cluster() {
  const [searchTerm, setSearchTerm] = useState("");

  // Mock data
  const nodes = [
    {
      name: "master-node-1",
      status: "Ready",
      roles: ["control-plane", "master"],
      age: "45d",
      version: "v1.28.2",
      internalIP: "192.168.1.10",
      externalIP: "203.0.113.10",
      os: "Ubuntu 22.04.3 LTS",
      kernel: "5.15.0-78-generic",
      containerRuntime: "containerd://1.7.2",
      cpuCapacity: "4",
      memoryCapacity: "8Gi",
      podCapacity: "110"
    },
    {
      name: "worker-node-1",
      status: "Ready",
      roles: ["worker"],
      age: "40d",
      version: "v1.28.2",
      internalIP: "192.168.1.11",
      externalIP: "203.0.113.11",
      os: "Ubuntu 22.04.3 LTS",
      kernel: "5.15.0-78-generic",
      containerRuntime: "containerd://1.7.2",
      cpuCapacity: "8",
      memoryCapacity: "16Gi",
      podCapacity: "110"
    },
    {
      name: "worker-node-2",
      status: "Ready",
      roles: ["worker"],
      age: "40d",
      version: "v1.28.2",
      internalIP: "192.168.1.12",
      externalIP: "203.0.113.12",
      os: "Ubuntu 22.04.3 LTS",
      kernel: "5.15.0-78-generic",
      containerRuntime: "containerd://1.7.2",
      cpuCapacity: "8",
      memoryCapacity: "16Gi",
      podCapacity: "110"
    },
    {
      name: "worker-node-3",
      status: "NotReady",
      roles: ["worker"],
      age: "35d",
      version: "v1.28.2",
      internalIP: "192.168.1.13",
      externalIP: "203.0.113.13",
      os: "Ubuntu 22.04.3 LTS",
      kernel: "5.15.0-78-generic",
      containerRuntime: "containerd://1.7.2",
      cpuCapacity: "8",
      memoryCapacity: "16Gi",
      podCapacity: "110"
    }
  ];

  const namespaces = [
    {
      name: "default",
      status: "Active",
      age: "45d"
    },
    {
      name: "kube-system",
      status: "Active",
      age: "45d"
    },
    {
      name: "kube-public",
      status: "Active",
      age: "45d"
    },
    {
      name: "kube-node-lease",
      status: "Active",
      age: "45d"
    },
    {
      name: "monitoring",
      status: "Active",
      age: "30d"
    },
    {
      name: "logging",
      status: "Active",
      age: "25d"
    },
    {
      name: "ingress-nginx",
      status: "Active",
      age: "20d"
    }
  ];

  const events = [
    {
      type: "Normal",
      reason: "Started",
      message: "Started container frontend-app",
      source: "kubelet",
      object: "pod/frontend-app-7d4b8c9f8d-xyz12",
      firstSeen: "5m",
      lastSeen: "5m",
      count: 1
    },
    {
      type: "Warning",
      reason: "FailedScheduling",
      message: "0/4 nodes are available: 1 node(s) had untolerated taint",
      source: "default-scheduler",
      object: "pod/database-migration-1a2b3c4d5e-jkl90",
      firstSeen: "30m",
      lastSeen: "25m",
      count: 12
    },
    {
      type: "Normal",
      reason: "Pulled",
      message: "Container image 'nginx:1.21' already present on machine",
      source: "kubelet",
      object: "pod/frontend-app-7d4b8c9f8d-abc34",
      firstSeen: "2h",
      lastSeen: "2h",
      count: 1
    },
    {
      type: "Warning",
      reason: "Unhealthy",
      message: "Readiness probe failed: Get http://10.244.1.22:8080/health: dial tcp 10.244.1.22:8080: connect: connection refused",
      source: "kubelet",
      object: "pod/backend-api-5f6a7b8c9d-def56",
      firstSeen: "1h",
      lastSeen: "45m",
      count: 5
    }
  ];

  const getStatusBadge = (status: string) => {
    switch (status) {
      case "Ready":
      case "Active":
        return (
          <Badge className="bg-emerald-100 dark:bg-emerald-950 text-emerald-800 dark:text-emerald-200 border-emerald-200 dark:border-emerald-800">
            <CheckCircle className="w-3 h-3 mr-1" />
            {status}
          </Badge>
        );
      case "NotReady":
        return (
          <Badge className="bg-red-100 dark:bg-red-950 text-red-800 dark:text-red-200 border-red-200 dark:border-red-800">
            <AlertTriangle className="w-3 h-3 mr-1" />
            Not Ready
          </Badge>
        );
      default:
        return <Badge variant="secondary">{status}</Badge>;
    }
  };

  const getEventTypeBadge = (type: string) => {
    switch (type) {
      case "Normal":
        return (
          <Badge className="bg-blue-100 dark:bg-blue-950 text-blue-800 dark:text-blue-200 border-blue-200 dark:border-blue-800">
            Normal
          </Badge>
        );
      case "Warning":
        return (
          <Badge className="bg-amber-100 dark:bg-amber-950 text-amber-800 dark:text-amber-200 border-amber-200 dark:border-amber-800">
            <AlertTriangle className="w-3 h-3 mr-1" />
            Warning
          </Badge>
        );
      default:
        return <Badge variant="secondary">{type}</Badge>;
    }
  };

  const filteredNodes = nodes.filter(node => 
    node.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const filteredNamespaces = namespaces.filter(ns => 
    ns.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const filteredEvents = events.filter(event => 
    event.reason.toLowerCase().includes(searchTerm.toLowerCase()) ||
    event.object.toLowerCase().includes(searchTerm.toLowerCase())
  );

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <div className="relative">
          <div className="flex items-center gap-4 mb-2">
            <div className="w-1 h-8 bg-gradient-to-b from-primary to-primary/80 rounded-full"></div>
            <h1 className="font-bold text-foreground">
              PULLPIRI Cluster
            </h1>
          </div>
          <p className="text-muted-foreground ml-8">
            Monitor and manage your Kubernetes cluster infrastructure
          </p>
        </div>
        <Button className="bg-primary hover:bg-primary/80 text-primary-foreground shadow-lg hover:shadow-xl transition-all gap-2">
          <Plus className="w-4 h-4" />
          Add Node
        </Button>
      </div>

      <div className="flex items-center gap-4">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search cluster resources..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="pl-10 h-12 bg-card/80 backdrop-blur-sm border-border/30 shadow-sm hover:shadow-md transition-all"
          />
        </div>
        <div className="flex items-center gap-3">
          <Badge className="bg-slate-50 dark:bg-slate-950 text-slate-700 dark:text-slate-300 border-slate-200 dark:border-slate-800 px-3 py-1">
            <Server className="w-3 h-3 mr-1" />
            {nodes.length} Nodes
          </Badge>
          <Badge className="bg-slate-50 dark:bg-slate-950 text-slate-700 dark:text-slate-300 border-slate-200 dark:border-slate-800 px-3 py-1">
            <Activity className="w-3 h-3 mr-1" />
            {namespaces.length} Namespaces
          </Badge>
        </div>
      </div>

      <Tabs defaultValue="nodes" className="space-y-6">
        <TabsList className="bg-card/80 backdrop-blur-sm border border-border/30 shadow-lg">
          <TabsTrigger value="nodes" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">
            Nodes
          </TabsTrigger>
          <TabsTrigger value="namespaces" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">
            Namespaces
          </TabsTrigger>
          <TabsTrigger value="events" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">
            Events
          </TabsTrigger>
        </TabsList>

        <TabsContent value="nodes">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                  <Server className="w-4 h-4 text-primary-foreground" />
                </div>
                <div>
                  <CardTitle className="text-foreground">Cluster Nodes</CardTitle>
                  <CardDescription>Physical and virtual machines in your cluster</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="overflow-hidden rounded-xl border border-border/30">
                <Table>
                  <TableHeader className="bg-muted/80">
                    <TableRow className="border-border/30">
                      <TableHead className="font-semibold text-foreground">Name</TableHead>
                      <TableHead className="font-semibold text-foreground">Status</TableHead>
                      <TableHead className="font-semibold text-foreground">Roles</TableHead>
                      <TableHead className="font-semibold text-foreground">Age</TableHead>
                      <TableHead className="font-semibold text-foreground">Version</TableHead>
                      <TableHead className="font-semibold text-foreground">Internal IP</TableHead>
                      <TableHead className="font-semibold text-foreground">CPU</TableHead>
                      <TableHead className="font-semibold text-foreground">Memory</TableHead>
                      <TableHead className="font-semibold text-foreground"></TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredNodes.map((node) => (
                      <TableRow key={node.name} className="border-border/30 hover:bg-muted/30 transition-colors">
                        <TableCell className="font-medium text-foreground">{node.name}</TableCell>
                        <TableCell>{getStatusBadge(node.status)}</TableCell>
                        <TableCell>
                          <div className="flex gap-1 flex-wrap">
                            {node.roles.map((role) => (
                              <Badge key={role} variant="secondary" className="text-xs">
                                {role}
                              </Badge>
                            ))}
                          </div>
                        </TableCell>
                        <TableCell className="text-muted-foreground">{node.age}</TableCell>
                        <TableCell className="font-mono text-sm">{node.version}</TableCell>
                        <TableCell className="font-mono text-sm text-muted-foreground">{node.internalIP}</TableCell>
                        <TableCell>
                          <div className="flex items-center gap-1">
                            <Cpu className="w-3 h-3 text-muted-foreground" />
                            <span className="font-mono text-sm">{node.cpuCapacity}</span>
                          </div>
                        </TableCell>
                        <TableCell>
                          <div className="flex items-center gap-1">
                            <MemoryStick className="w-3 h-3 text-muted-foreground" />
                            <span className="font-mono text-sm">{node.memoryCapacity}</span>
                          </div>
                        </TableCell>
                        <TableCell>
                          <Button variant="ghost" size="sm" className="w-8 h-8 hover:bg-muted">
                            <MoreHorizontal className="h-3 w-3" />
                          </Button>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="namespaces">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                  <Activity className="w-4 h-4 text-primary-foreground" />
                </div>
                <div>
                  <CardTitle className="text-foreground">Namespaces</CardTitle>
                  <CardDescription>Logical partitions within your cluster</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="overflow-hidden rounded-xl border border-border/30">
                <Table>
                  <TableHeader className="bg-muted/80">
                    <TableRow className="border-border/30">
                      <TableHead className="font-semibold text-foreground">Name</TableHead>
                      <TableHead className="font-semibold text-foreground">Status</TableHead>
                      <TableHead className="font-semibold text-foreground">Age</TableHead>
                      <TableHead className="font-semibold text-foreground"></TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredNamespaces.map((namespace) => (
                      <TableRow key={namespace.name} className="border-border/30 hover:bg-muted/30 transition-colors">
                        <TableCell className="font-medium text-foreground">{namespace.name}</TableCell>
                        <TableCell>{getStatusBadge(namespace.status)}</TableCell>
                        <TableCell className="text-muted-foreground">{namespace.age}</TableCell>
                        <TableCell>
                          <Button variant="ghost" size="sm" className="w-8 h-8 hover:bg-muted">
                            <MoreHorizontal className="h-3 w-3" />
                          </Button>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="events">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                  <Activity className="w-4 h-4 text-primary-foreground" />
                </div>
                <div>
                  <CardTitle className="text-foreground">Cluster Events</CardTitle>
                  <CardDescription>Recent activities and alerts in your cluster</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="overflow-hidden rounded-xl border border-border/30">
                <Table>
                  <TableHeader className="bg-muted/80">
                    <TableRow className="border-border/30">
                      <TableHead className="font-semibold text-foreground">Type</TableHead>
                      <TableHead className="font-semibold text-foreground">Reason</TableHead>
                      <TableHead className="font-semibold text-foreground">Object</TableHead>
                      <TableHead className="font-semibold text-foreground">Message</TableHead>
                      <TableHead className="font-semibold text-foreground">Source</TableHead>
                      <TableHead className="font-semibold text-foreground">First Seen</TableHead>
                      <TableHead className="font-semibold text-foreground">Count</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredEvents.map((event, index) => (
                      <TableRow key={index} className="border-border/30 hover:bg-muted/30 transition-colors">
                        <TableCell>{getEventTypeBadge(event.type)}</TableCell>
                        <TableCell className="font-medium text-foreground">{event.reason}</TableCell>
                        <TableCell className="font-mono text-sm text-muted-foreground max-w-xs truncate" title={event.object}>
                          {event.object}
                        </TableCell>
                        <TableCell className="text-sm max-w-md truncate" title={event.message}>
                          {event.message}
                        </TableCell>
                        <TableCell className="text-sm text-muted-foreground">{event.source}</TableCell>
                        <TableCell className="text-muted-foreground">{event.firstSeen}</TableCell>
                        <TableCell>
                          <Badge variant="outline" className="text-xs">
                            {event.count}
                          </Badge>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}