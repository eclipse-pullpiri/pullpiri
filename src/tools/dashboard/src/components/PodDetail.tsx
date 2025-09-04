import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "./ui/table";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Separator } from "./ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { ScrollArea } from "./ui/scroll-area";
import { 
  ArrowLeft, 
  Box, 
  Copy, 
  Download, 
  Play, 
  Square, 
  RotateCcw, 
  Terminal, 
  FileText, 
  Activity,
  Clock,
  Server,
  Cpu,
  MemoryStick,
  Network,
  AlertCircle
} from "lucide-react";

interface PodDetailProps {
  podName: string;
  podData?: {
    name: string;
    image: string;
    labels: Record<string, string>;
    node: string;
    status: string;
    cpuUsage: string;
    memoryUsage: string;
    age: string;
    ready: string;
    restarts: number;
    ip: string;
  };
  onBack: () => void;
}

export function PodDetail({ podName, podData, onBack }: PodDetailProps) {
  // Use actual pod data if available, otherwise fall back to mock data
  const podInfo = podData ? {
    name: podData.name,
    namespace: "default",
    status: podData.status,
    restarts: podData.restarts,
    age: podData.age,
    node: podData.node,
    ip: podData.ip,
    image: podData.image,
    cpuUsage: podData.cpuUsage,
    memoryUsage: podData.memoryUsage,
    labels: podData.labels,
    annotations: {
      "deployment.kubernetes.io/revision": "3",
      "kubectl.kubernetes.io/last-applied-configuration": "...",
      "prometheus.io/scrape": "true",
      "prometheus.io/port": "8080"
    }
  } : {
    // Fallback to mock data if podData is not available
    name: podName,
    namespace: "default",
    status: "Running",
    restarts: podName.includes("backend") ? 1 : 0,
    age: podName.includes("frontend") ? "2d" : podName.includes("backend") ? "5d" : podName.includes("redis") ? "1d" : "30m",
    node: podName.includes("xyz12") ? "worker-node-1" : podName.includes("abc34") ? "worker-node-2" : podName.includes("def56") ? "worker-node-1" : podName.includes("ghi78") ? "worker-node-3" : "worker-node-2",
    ip: podName.includes("xyz12") ? "10.244.1.15" : podName.includes("abc34") ? "10.244.2.18" : podName.includes("def56") ? "10.244.1.22" : podName.includes("ghi78") ? "10.244.3.9" : "N/A",
    image: podName.includes("frontend") ? "nginx:1.21" : podName.includes("backend") ? "node:18-alpine" : podName.includes("redis") ? "redis:7-alpine" : "postgres:14",
    cpuUsage: podName.includes("backend") ? "120m" : podName.includes("frontend") ? "45m" : podName.includes("redis") ? "25m" : "0m",
    memoryUsage: podName.includes("backend") ? "256Mi" : podName.includes("frontend") ? "128Mi" : podName.includes("redis") ? "64Mi" : "0Mi",
    labels: podName.includes("frontend") 
      ? { app: "frontend", version: "v1.2.0", tier: "web" }
      : podName.includes("backend") 
      ? { app: "backend", tier: "api", version: "v1.0.0" }
      : podName.includes("redis")
      ? { app: "redis", role: "cache", version: "v7.0" }
      : { job: "migration", app: "database" },
    annotations: {
      "deployment.kubernetes.io/revision": "3",
      "kubectl.kubernetes.io/last-applied-configuration": "...",
      "prometheus.io/scrape": "true",
      "prometheus.io/port": "8080"
    }
  };

  const containers = [
    {
      name: podName.includes("frontend") ? "nginx" : podName.includes("backend") ? "api-server" : podName.includes("redis") ? "redis" : "postgres",
      image: podInfo.image,
      ready: podInfo.status === "Running",
      restartCount: podInfo.restarts,
      state: podInfo.status === "Running" ? "Running" : "Pending",
      started: podInfo.age,
      ports: podName.includes("frontend") 
        ? [{ containerPort: 80, protocol: "TCP" }, { containerPort: 443, protocol: "TCP" }]
        : podName.includes("backend") 
        ? [{ containerPort: 8080, protocol: "TCP" }, { containerPort: 9090, protocol: "TCP" }]
        : podName.includes("redis")
        ? [{ containerPort: 6379, protocol: "TCP" }]
        : [{ containerPort: 5432, protocol: "TCP" }],
      env: podName.includes("frontend")
        ? [
            { name: "NGINX_PORT", value: "80" },
            { name: "NGINX_HOST", value: "localhost" }
          ]
        : podName.includes("backend")
        ? [
            { name: "NODE_ENV", value: "production" },
            { name: "PORT", value: "8080" },
            { name: "DATABASE_URL", value: "postgresql://..." }
          ]
        : podName.includes("redis")
        ? [
            { name: "REDIS_PASSWORD", value: "***" },
            { name: "REDIS_PORT", value: "6379" }
          ]
        : [
            { name: "POSTGRES_DB", value: "myapp" },
            { name: "POSTGRES_USER", value: "user" },
            { name: "POSTGRES_PASSWORD", value: "***" }
          ],
      volumeMounts: podName.includes("frontend")
        ? [
            { name: "nginx-config", mountPath: "/etc/nginx/conf.d", readOnly: true },
            { name: "static-content", mountPath: "/usr/share/nginx/html", readOnly: false }
          ]
        : podName.includes("backend")
        ? [
            { name: "app-config", mountPath: "/app/config", readOnly: true },
            { name: "logs", mountPath: "/app/logs", readOnly: false }
          ]
        : podName.includes("redis")
        ? [
            { name: "redis-data", mountPath: "/data", readOnly: false },
            { name: "redis-config", mountPath: "/etc/redis", readOnly: true }
          ]
        : [
            { name: "postgres-data", mountPath: "/var/lib/postgresql/data", readOnly: false },
            { name: "postgres-config", mountPath: "/etc/postgresql", readOnly: true }
          ],
      resources: {
        requests: {
          cpu: podName.includes("backend") ? "200m" : podName.includes("frontend") ? "100m" : podName.includes("redis") ? "50m" : "100m",
          memory: podName.includes("backend") ? "256Mi" : podName.includes("frontend") ? "128Mi" : podName.includes("redis") ? "64Mi" : "256Mi"
        },
        limits: {
          cpu: podName.includes("backend") ? "500m" : podName.includes("frontend") ? "200m" : podName.includes("redis") ? "100m" : "500m",
          memory: podName.includes("backend") ? "512Mi" : podName.includes("frontend") ? "256Mi" : podName.includes("redis") ? "128Mi" : "512Mi"
        }
      }
    }
  ];

  const conditions = [
    {
      type: "Initialized",
      status: "True",
      lastProbeTime: "null",
      lastTransitionTime: "2024-01-15T10:30:45Z",
      reason: "PodCompleted"
    },
    {
      type: "Ready",
      status: podInfo.status === "Running" ? "True" : "False",
      lastProbeTime: "null",
      lastTransitionTime: "2024-01-15T10:31:02Z",
      reason: "ContainersReady"
    },
    {
      type: "ContainersReady",
      status: podInfo.status === "Running" ? "True" : "False",
      lastProbeTime: "null",
      lastTransitionTime: "2024-01-15T10:31:02Z",
      reason: "ContainersReady"
    },
    {
      type: "PodScheduled",
      status: "True",
      lastProbeTime: "null",
      lastTransitionTime: "2024-01-15T10:30:45Z",
      reason: "PodScheduled"
    }
  ];

  const events = [
    {
      type: "Normal",
      reason: "Scheduled",
      message: `Successfully assigned default/${podName} to ${podInfo.node}`,
      firstTimestamp: "5m ago",
      count: 1
    },
    {
      type: "Normal",
      reason: "Pulled",
      message: `Container image "${podInfo.image}" already present on machine`,
      firstTimestamp: "5m ago",
      count: 1
    },
    {
      type: "Normal",
      reason: "Created",
      message: "Created container",
      firstTimestamp: "5m ago",
      count: 1
    },
    {
      type: "Normal",
      reason: "Started",
      message: "Started container",
      firstTimestamp: "5m ago",
      count: 1
    }
  ];

  const logs = [
    "2024-01-15T10:31:02.123Z [INFO] Application starting...",
    "2024-01-15T10:31:02.456Z [INFO] Loading configuration from /etc/config",
    "2024-01-15T10:31:02.789Z [INFO] Database connection established",
    "2024-01-15T10:31:03.012Z [INFO] HTTP server listening on port 8080",
    "2024-01-15T10:31:03.345Z [INFO] Health check endpoint ready",
    "2024-01-15T10:31:03.678Z [INFO] Application ready to serve requests",
    "2024-01-15T10:31:04.901Z [INFO] Processing request GET /api/health",
    "2024-01-15T10:31:05.234Z [INFO] Request completed in 12ms",
    "2024-01-15T10:31:10.567Z [INFO] Processing request GET /api/status",
    "2024-01-15T10:31:10.890Z [INFO] Request completed in 8ms"
  ];

  const getStatusBadge = (status: string) => {
    switch (status) {
      case "Running":
        return (
          <Badge className="bg-emerald-100 dark:bg-emerald-950 text-emerald-800 dark:text-emerald-200 border-emerald-200 dark:border-emerald-800">
            <div className="w-2 h-2 bg-emerald-500 rounded-full mr-1 animate-pulse"></div>
            Running
          </Badge>
        );
      case "Pending":
        return (
          <Badge className="bg-amber-100 dark:bg-amber-950 text-amber-800 dark:text-amber-200 border-amber-200 dark:border-amber-800">
            <div className="w-2 h-2 bg-amber-500 rounded-full mr-1"></div>
            Pending
          </Badge>
        );
      case "Failed":
        return (
          <Badge className="bg-red-100 dark:bg-red-950 text-red-800 dark:text-red-200 border-red-200 dark:border-red-800">
            <AlertCircle className="w-3 h-3 mr-1" />
            Failed
          </Badge>
        );
      default:
        return <Badge variant="secondary">{status}</Badge>;
    }
  };

  const getConditionBadge = (status: string) => {
    return status === "True" ? (
      <Badge className="bg-emerald-100 dark:bg-emerald-950 text-emerald-800 dark:text-emerald-200">
        True
      </Badge>
    ) : (
      <Badge className="bg-red-100 dark:bg-red-950 text-red-800 dark:text-red-200">
        False
      </Badge>
    );
  };

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Button
            variant="ghost"
            size="sm"
            onClick={onBack}
            className="p-2 hover:bg-muted rounded-lg"
          >
            <ArrowLeft className="w-4 h-4" />
          </Button>
          <div className="relative">
            <div className="flex items-center gap-4 mb-2">
              <div className="w-1 h-8 bg-gradient-to-b from-primary to-primary/80 rounded-full"></div>
              <h1 className="font-bold text-foreground">
                Pod Details
              </h1>
            </div>
            <p className="text-muted-foreground ml-8">
              <span className="font-semibold text-primary">{podName}</span> in namespace <span className="font-semibold text-primary">"{podInfo.namespace}"</span>
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" className="gap-2">
            <Terminal className="w-4 h-4" />
            Exec
          </Button>
          <Button variant="outline" size="sm" className="gap-2">
            <Download className="w-4 h-4" />
            Download Logs
          </Button>
          <Button variant="outline" size="sm" className="gap-2">
            <RotateCcw className="w-4 h-4" />
            Restart
          </Button>
        </div>
      </div>

      {/* Pod Overview Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm">Status</CardTitle>
              <Activity className="w-4 h-4 text-muted-foreground" />
            </div>
          </CardHeader>
          <CardContent className="pt-0">
            {getStatusBadge(podInfo.status)}
            <p className="text-xs text-muted-foreground mt-2">
              {podInfo.restarts} restarts
            </p>
          </CardContent>
        </Card>

        <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm">Node</CardTitle>
              <Server className="w-4 h-4 text-muted-foreground" />
            </div>
          </CardHeader>
          <CardContent className="pt-0">
            <p className="font-mono text-sm">{podInfo.node}</p>
            <p className="text-xs text-muted-foreground mt-1">
              IP: {podInfo.ip}
            </p>
          </CardContent>
        </Card>

        <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm">CPU Usage</CardTitle>
              <Cpu className="w-4 h-4 text-muted-foreground" />
            </div>
          </CardHeader>
          <CardContent className="pt-0">
            <p className="font-mono text-lg font-semibold">{podInfo.cpuUsage}</p>
            <p className="text-xs text-muted-foreground">
              Current usage
            </p>
          </CardContent>
        </Card>

        <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm">Memory Usage</CardTitle>
              <MemoryStick className="w-4 h-4 text-muted-foreground" />
            </div>
          </CardHeader>
          <CardContent className="pt-0">
            <p className="font-mono text-lg font-semibold">{podInfo.memoryUsage}</p>
            <p className="text-xs text-muted-foreground">
              Current usage
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Overview Section */}
      <div>
        <div className="flex items-center gap-3 mb-6">
          <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
            <Activity className="w-4 h-4 text-primary-foreground" />
          </div>
          <div>
            <h2 className="text-foreground">Overview</h2>
            <p className="text-muted-foreground text-sm">Basic information and pod metadata</p>
          </div>
        </div>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <CardTitle className="text-foreground">Basic Information</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid grid-cols-3 gap-4 text-sm">
                <div>
                  <p className="text-muted-foreground">Name</p>
                  <p className="font-mono">{podInfo.name}</p>
                </div>
                <div>
                  <p className="text-muted-foreground">Namespace</p>
                  <p className="font-mono">{podInfo.namespace}</p>
                </div>
                <div>
                  <p className="text-muted-foreground">Age</p>
                  <p className="font-mono">{podInfo.age}</p>
                </div>
                <div>
                  <p className="text-muted-foreground">Node</p>
                  <p className="font-mono">{podInfo.node}</p>
                </div>
                <div>
                  <p className="text-muted-foreground">Pod IP</p>
                  <p className="font-mono">{podInfo.ip}</p>
                </div>
                <div>
                  <p className="text-muted-foreground">Restarts</p>
                  <p className="font-mono">{podInfo.restarts}</p>
                </div>
              </div>
            </CardContent>
          </Card>

          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <CardTitle className="text-foreground">Labels</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex flex-wrap gap-2">
                {Object.entries(podInfo.labels).map(([key, value]) => (
                  <Badge key={key} variant="secondary" className="text-xs">
                    {key}={value}
                  </Badge>
                ))}
              </div>
            </CardContent>
          </Card>

          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl lg:col-span-2">
            <CardHeader>
              <CardTitle className="text-foreground">Annotations</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                {Object.entries(podInfo.annotations).map(([key, value]) => (
                  <div key={key} className="flex items-center justify-between py-2 border-b border-border/20 last:border-0">
                    <span className="text-sm font-mono text-muted-foreground">{key}</span>
                    <span className="text-sm font-mono max-w-md truncate" title={value}>{value}</span>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </div>
      </div>

      {/* Containers Section */}
      <div>
        <div className="flex items-center gap-3 mb-6">
          <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
            <Box className="w-4 h-4 text-primary-foreground" />
          </div>
          <div>
            <h2 className="text-foreground">Containers</h2>
            <p className="text-muted-foreground text-sm">Container instances running in this pod</p>
          </div>
        </div>
        <div className="space-y-6">
          {containers.map((container, index) => (
            <Card key={container.name} className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
              <CardHeader>
                <div className="flex items-center gap-3">
                  <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                    <Box className="w-4 h-4 text-primary-foreground" />
                  </div>
                  <div className="flex-1">
                    <CardTitle className="text-foreground flex items-center gap-3">
                      {container.name}
                      <Badge className={container.ready 
                        ? "bg-emerald-100 dark:bg-emerald-950 text-emerald-800 dark:text-emerald-200"
                        : "bg-red-100 dark:bg-red-950 text-red-800 dark:text-red-200"
                      }>
                        {container.state}
                      </Badge>
                    </CardTitle>
                    <CardDescription className="font-mono text-sm">
                      {container.image}
                    </CardDescription>
                  </div>
                </div>
              </CardHeader>
              <CardContent className="space-y-6">
                {/* Basic Container Info */}
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4 p-4 bg-muted/20 rounded-lg">
                  <div>
                    <p className="text-xs text-muted-foreground">Status</p>
                    <p className="font-mono text-sm">{container.ready ? "Ready" : "Not Ready"}</p>
                  </div>
                  <div>
                    <p className="text-xs text-muted-foreground">Restarts</p>
                    <p className="font-mono text-sm">{container.restartCount}</p>
                  </div>
                  <div>
                    <p className="text-xs text-muted-foreground">Started</p>
                    <p className="font-mono text-sm">{container.started}</p>
                  </div>
                  <div>
                    <p className="text-xs text-muted-foreground">State</p>
                    <p className="font-mono text-sm">{container.state}</p>
                  </div>
                </div>

                {/* Container Details Tabs */}
                <Tabs defaultValue="ports" className="w-full">
                  <TabsList className="grid w-full grid-cols-4">
                    <TabsTrigger value="ports">Ports</TabsTrigger>
                    <TabsTrigger value="env">Environment</TabsTrigger>
                    <TabsTrigger value="volumes">Volumes</TabsTrigger>
                    <TabsTrigger value="resources">Resources</TabsTrigger>
                  </TabsList>

                  <TabsContent value="ports" className="mt-4">
                    <div className="space-y-3">
                      <h4 className="font-medium text-foreground">Port Configuration</h4>
                      <div className="overflow-hidden rounded-lg border border-border/30">
                        <Table>
                          <TableHeader className="bg-muted/50">
                            <TableRow>
                              <TableHead>Container Port</TableHead>
                              <TableHead>Protocol</TableHead>
                              <TableHead>Name</TableHead>
                            </TableRow>
                          </TableHeader>
                          <TableBody>
                            {container.ports.map((port, portIndex) => (
                              <TableRow key={portIndex}>
                                <TableCell className="font-mono">{port.containerPort}</TableCell>
                                <TableCell>
                                  <Badge variant="outline">{port.protocol}</Badge>
                                </TableCell>
                                <TableCell className="text-muted-foreground">
                                  {port.containerPort === 80 ? "http" : 
                                   port.containerPort === 443 ? "https" :
                                   port.containerPort === 8080 ? "api" :
                                   port.containerPort === 9090 ? "metrics" :
                                   port.containerPort === 6379 ? "redis" :
                                   port.containerPort === 5432 ? "postgres" : "-"}
                                </TableCell>
                              </TableRow>
                            ))}
                          </TableBody>
                        </Table>
                      </div>
                    </div>
                  </TabsContent>

                  <TabsContent value="env" className="mt-4">
                    <div className="space-y-3">
                      <h4 className="font-medium text-foreground">Environment Variables</h4>
                      <div className="overflow-hidden rounded-lg border border-border/30">
                        <Table>
                          <TableHeader className="bg-muted/50">
                            <TableRow>
                              <TableHead>Name</TableHead>
                              <TableHead>Value</TableHead>
                            </TableRow>
                          </TableHeader>
                          <TableBody>
                            {container.env.map((env, envIndex) => (
                              <TableRow key={envIndex}>
                                <TableCell className="font-mono text-sm">{env.name}</TableCell>
                                <TableCell className="font-mono text-sm">
                                  {env.value.includes("***") ? (
                                    <span className="text-muted-foreground italic">***hidden***</span>
                                  ) : (
                                    env.value
                                  )}
                                </TableCell>
                              </TableRow>
                            ))}
                          </TableBody>
                        </Table>
                      </div>
                    </div>
                  </TabsContent>

                  <TabsContent value="volumes" className="mt-4">
                    <div className="space-y-3">
                      <h4 className="font-medium text-foreground">Volume Mounts</h4>
                      <div className="overflow-hidden rounded-lg border border-border/30">
                        <Table>
                          <TableHeader className="bg-muted/50">
                            <TableRow>
                              <TableHead>Name</TableHead>
                              <TableHead>Mount Path</TableHead>
                              <TableHead>Read Only</TableHead>
                            </TableRow>
                          </TableHeader>
                          <TableBody>
                            {container.volumeMounts.map((mount, mountIndex) => (
                              <TableRow key={mountIndex}>
                                <TableCell className="font-mono text-sm">{mount.name}</TableCell>
                                <TableCell className="font-mono text-sm">{mount.mountPath}</TableCell>
                                <TableCell>
                                  <Badge variant={mount.readOnly ? "secondary" : "outline"}>
                                    {mount.readOnly ? "Read Only" : "Read/Write"}
                                  </Badge>
                                </TableCell>
                              </TableRow>
                            ))}
                          </TableBody>
                        </Table>
                      </div>
                    </div>
                  </TabsContent>

                  <TabsContent value="resources" className="mt-4">
                    <div className="space-y-4">
                      <h4 className="font-medium text-foreground">Resource Configuration</h4>
                      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <div className="p-4 bg-muted/20 rounded-lg">
                          <h5 className="text-sm font-medium text-foreground mb-3 flex items-center gap-2">
                            <Cpu className="w-4 h-4" />
                            CPU Resources
                          </h5>
                          <div className="space-y-2">
                            <div className="flex justify-between">
                              <span className="text-xs text-muted-foreground">Request:</span>
                              <span className="font-mono text-sm">{container.resources.requests.cpu}</span>
                            </div>
                            <div className="flex justify-between">
                              <span className="text-xs text-muted-foreground">Limit:</span>
                              <span className="font-mono text-sm">{container.resources.limits.cpu}</span>
                            </div>
                          </div>
                        </div>
                        <div className="p-4 bg-muted/20 rounded-lg">
                          <h5 className="text-sm font-medium text-foreground mb-3 flex items-center gap-2">
                            <MemoryStick className="w-4 h-4" />
                            Memory Resources
                          </h5>
                          <div className="space-y-2">
                            <div className="flex justify-between">
                              <span className="text-xs text-muted-foreground">Request:</span>
                              <span className="font-mono text-sm">{container.resources.requests.memory}</span>
                            </div>
                            <div className="flex justify-between">
                              <span className="text-xs text-muted-foreground">Limit:</span>
                              <span className="font-mono text-sm">{container.resources.limits.memory}</span>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </TabsContent>
                </Tabs>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>

      {/* Conditions Section */}
      <div>
        <div className="flex items-center gap-3 mb-6">
          <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
            <Activity className="w-4 h-4 text-primary-foreground" />
          </div>
          <div>
            <h2 className="text-foreground">Pod Conditions</h2>
            <p className="text-muted-foreground text-sm">Current status conditions of the pod</p>
          </div>
        </div>
        <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
          <CardContent className="pt-6">
            <div className="overflow-hidden rounded-xl border border-border/30">
              <Table>
                <TableHeader className="bg-muted/80">
                  <TableRow className="border-border/30">
                    <TableHead className="font-semibold text-foreground">Type</TableHead>
                    <TableHead className="font-semibold text-foreground">Status</TableHead>
                    <TableHead className="font-semibold text-foreground">Last Transition Time</TableHead>
                    <TableHead className="font-semibold text-foreground">Reason</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {conditions.map((condition) => (
                    <TableRow key={condition.type} className="border-border/30">
                      <TableCell className="font-medium text-foreground">{condition.type}</TableCell>
                      <TableCell>{getConditionBadge(condition.status)}</TableCell>
                      <TableCell className="font-mono text-sm text-muted-foreground">{condition.lastTransitionTime}</TableCell>
                      <TableCell className="text-sm">{condition.reason}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Events Section */}
      <div>
        <div className="flex items-center gap-3 mb-6">
          <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
            <Clock className="w-4 h-4 text-primary-foreground" />
          </div>
          <div>
            <h2 className="text-foreground">Recent Events</h2>
            <p className="text-muted-foreground text-sm">Events related to this pod</p>
          </div>
        </div>
        <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
          <CardContent className="pt-6">
            <div className="overflow-hidden rounded-xl border border-border/30">
              <Table>
                <TableHeader className="bg-muted/80">
                  <TableRow className="border-border/30">
                    <TableHead className="font-semibold text-foreground">Type</TableHead>
                    <TableHead className="font-semibold text-foreground">Reason</TableHead>
                    <TableHead className="font-semibold text-foreground">Message</TableHead>
                    <TableHead className="font-semibold text-foreground">First Seen</TableHead>
                    <TableHead className="font-semibold text-foreground">Count</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {events.map((event, index) => (
                    <TableRow key={index} className="border-border/30">
                      <TableCell>
                        <Badge className="bg-blue-100 dark:bg-blue-950 text-blue-800 dark:text-blue-200">
                          {event.type}
                        </Badge>
                      </TableCell>
                      <TableCell className="font-medium text-foreground">{event.reason}</TableCell>
                      <TableCell className="text-sm max-w-md truncate" title={event.message}>{event.message}</TableCell>
                      <TableCell className="text-muted-foreground">{event.firstTimestamp}</TableCell>
                      <TableCell>
                        <Badge variant="outline" className="text-xs">{event.count}</Badge>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Logs Section */}
      <div>
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
              <FileText className="w-4 h-4 text-primary-foreground" />
            </div>
            <div>
              <h2 className="text-foreground">Container Logs</h2>
              <p className="text-muted-foreground text-sm">Recent log output from the pod containers</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" className="gap-2">
              <Copy className="w-4 h-4" />
              Copy
            </Button>
            <Button variant="outline" size="sm" className="gap-2">
              <Download className="w-4 h-4" />
              Download
            </Button>
          </div>
        </div>
        <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
          <CardContent className="pt-6">
            <ScrollArea className="h-96 w-full rounded-xl border border-border/30 bg-muted/20 p-4">
              <div className="space-y-1 font-mono text-sm">
                {logs.map((log, index) => (
                  <div key={index} className="text-muted-foreground hover:text-foreground transition-colors">
                    {log}
                  </div>
                ))}
              </div>
            </ScrollArea>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}