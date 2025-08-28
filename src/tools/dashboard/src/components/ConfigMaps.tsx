import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "./ui/table";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Search, MoreHorizontal, Eye, Download, Database } from "lucide-react";

interface ConfigMapsProps {
  namespace: string;
}

export function ConfigMaps({ namespace }: ConfigMapsProps) {
  const [searchTerm, setSearchTerm] = useState("");

  // Mock data
  const configMaps = [
    {
      name: "app-config",
      keys: "app.properties, database.yaml",
      age: "2d"
    },
    {
      name: "nginx-config", 
      keys: "nginx.conf, ssl.conf",
      age: "5d"
    },
    {
      name: "redis-config",
      keys: "redis.conf",
      age: "1d"
    },
    {
      name: "monitoring-config",
      keys: "prometheus.yml, grafana-dashboard.json",
      age: "7d"
    }
  ];

  const secrets = [
    {
      name: "database-credentials",
      type: "Opaque",
      keys: "username, password",
      age: "5d"
    },
    {
      name: "tls-certificate",
      type: "kubernetes.io/tls", 
      keys: "tls.crt, tls.key",
      age: "30d"
    },
    {
      name: "registry-secret",
      type: "kubernetes.io/dockerconfigjson",
      keys: ".dockerconfigjson",
      age: "10d"
    },
    {
      name: "api-keys",
      type: "Opaque",
      keys: "external-api-key, webhook-secret",
      age: "2d"
    }
  ];

  const persistentVolumes = [
    {
      name: "postgres-pv",
      capacity: "10Gi",
      accessModes: "RWO",
      reclaimPolicy: "Retain",
      status: "Bound",
      claim: "default/postgres-pvc",
      storageClass: "standard",
      age: "15d"
    },
    {
      name: "redis-pv",
      capacity: "5Gi", 
      accessModes: "RWO",
      reclaimPolicy: "Delete",
      status: "Bound",
      claim: "default/redis-pvc",
      storageClass: "fast-ssd",
      age: "7d"
    },
    {
      name: "logs-pv",
      capacity: "50Gi",
      accessModes: "RWX",
      reclaimPolicy: "Retain", 
      status: "Available",
      claim: "N/A",
      storageClass: "standard",
      age: "30d"
    }
  ];

  const persistentVolumeClaims = [
    {
      name: "postgres-pvc",
      status: "Bound",
      volume: "postgres-pv", 
      capacity: "10Gi",
      accessModes: "RWO",
      storageClass: "standard",
      age: "15d"
    },
    {
      name: "redis-pvc",
      status: "Bound",
      volume: "redis-pv",
      capacity: "5Gi",
      accessModes: "RWO", 
      storageClass: "fast-ssd",
      age: "7d"
    },
    {
      name: "backup-pvc",
      status: "Pending",
      volume: "N/A",
      capacity: "20Gi",
      accessModes: "RWO",
      storageClass: "standard",
      age: "5m"
    }
  ];

  const getStatusBadge = (status: string) => {
    switch (status) {
      case "Bound":
        return <Badge className="bg-green-100 dark:bg-green-950 text-green-800 dark:text-green-200 hover:bg-green-100 dark:hover:bg-green-950">Bound</Badge>;
      case "Available":
        return <Badge className="bg-blue-100 dark:bg-blue-950 text-blue-800 dark:text-blue-200 hover:bg-blue-100 dark:hover:bg-blue-950">Available</Badge>;
      case "Pending":
        return <Badge className="bg-yellow-100 dark:bg-yellow-950 text-yellow-800 dark:text-yellow-200 hover:bg-yellow-100 dark:hover:bg-yellow-950">Pending</Badge>;
      case "Failed":
        return <Badge className="bg-red-100 dark:bg-red-950 text-red-800 dark:text-red-200 hover:bg-red-100 dark:hover:bg-red-950">Failed</Badge>;
      default:
        return <Badge variant="secondary">{status}</Badge>;
    }
  };

  const getSecretTypeBadge = (type: string) => {
    const shortType = type.replace("kubernetes.io/", "");
    return <Badge variant="outline" className="font-mono text-xs">{shortType}</Badge>;
  };

  const filteredConfigMaps = configMaps.filter(cm => 
    cm.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const filteredSecrets = secrets.filter(secret => 
    secret.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const filteredPVs = persistentVolumes.filter(pv => 
    pv.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const filteredPVCs = persistentVolumeClaims.filter(pvc => 
    pvc.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="relative">
          <div className="flex items-center gap-4 mb-2">
            <div className="w-1 h-8 bg-gradient-to-b from-primary to-primary/80 rounded-full"></div>
            <h1 className="font-bold text-foreground">
              PULLPIRI Config & Storage
            </h1>
          </div>
          <p className="text-muted-foreground ml-8">
            Manage configuration, secrets, and storage resources in <span className="font-semibold text-primary">"{namespace}"</span>
          </p>
        </div>
        <Button className="bg-primary hover:bg-primary/80 text-primary-foreground shadow-lg hover:shadow-xl transition-all">
          Create ConfigMap
        </Button>
      </div>

      <div className="flex items-center gap-4">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search resources..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="pl-10 h-12 bg-card/80 backdrop-blur-sm border-border/30 shadow-sm hover:shadow-md transition-all"
          />
        </div>
      </div>

      <Tabs defaultValue="configmaps" className="space-y-6">
        <TabsList className="bg-card/80 backdrop-blur-sm border border-border/30 shadow-lg">
          <TabsTrigger value="configmaps" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">ConfigMaps</TabsTrigger>
          <TabsTrigger value="secrets" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">Secrets</TabsTrigger>
          <TabsTrigger value="pvs" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">Persistent Volumes</TabsTrigger>
          <TabsTrigger value="pvcs" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">Volume Claims</TabsTrigger>
        </TabsList>

        <TabsContent value="configmaps">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                  <Database className="w-4 h-4 text-primary-foreground" />
                </div>
                <div>
                  <CardTitle className="text-foreground">ConfigMaps</CardTitle>
                  <CardDescription>Store configuration data as key-value pairs</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="overflow-hidden rounded-xl border border-border/30">
                <Table>
                  <TableHeader className="bg-muted/80">
                    <TableRow className="border-border/30">
                      <TableHead className="font-semibold text-foreground">Name</TableHead>
                      <TableHead className="font-semibold text-foreground">Data Keys</TableHead>
                      <TableHead className="font-semibold text-foreground">Age</TableHead>
                      <TableHead className="font-semibold text-foreground"></TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredConfigMaps.map((configMap) => (
                      <TableRow key={configMap.name} className="border-border/30 hover:bg-muted/30 transition-colors">
                        <TableCell className="font-medium text-foreground">{configMap.name}</TableCell>
                        <TableCell className="text-sm text-muted-foreground max-w-md truncate" title={configMap.keys}>
                          {configMap.keys}
                        </TableCell>
                        <TableCell className="text-muted-foreground">{configMap.age}</TableCell>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <Button variant="ghost" size="sm" className="w-8 h-8 hover:bg-muted">
                              <Eye className="h-4 w-4" />
                            </Button>
                            <Button variant="ghost" size="sm" className="w-8 h-8 hover:bg-muted">
                              <Download className="h-4 w-4" />
                            </Button>
                            <Button variant="ghost" size="sm" className="w-8 h-8 hover:bg-muted">
                              <MoreHorizontal className="h-4 w-4" />
                            </Button>
                          </div>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="secrets">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <CardTitle className="text-foreground">Secrets</CardTitle>
              <CardDescription>Store sensitive data such as passwords and API keys</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="overflow-hidden rounded-xl border border-border/30">
                <Table>
                  <TableHeader className="bg-muted/80">
                    <TableRow className="border-border/30">
                      <TableHead className="font-semibold text-foreground">Name</TableHead>
                      <TableHead className="font-semibold text-foreground">Type</TableHead>
                      <TableHead className="font-semibold text-foreground">Data Keys</TableHead>
                      <TableHead className="font-semibold text-foreground">Age</TableHead>
                      <TableHead className="font-semibold text-foreground"></TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredSecrets.map((secret) => (
                      <TableRow key={secret.name} className="border-border/30 hover:bg-muted/30 transition-colors">
                        <TableCell className="font-medium text-foreground">{secret.name}</TableCell>
                        <TableCell>{getSecretTypeBadge(secret.type)}</TableCell>
                        <TableCell className="text-sm text-muted-foreground">{secret.keys}</TableCell>
                        <TableCell className="text-muted-foreground">{secret.age}</TableCell>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <Button variant="ghost" size="sm" className="w-8 h-8 hover:bg-muted">
                              <Eye className="h-4 w-4" />
                            </Button>
                            <Button variant="ghost" size="sm" className="w-8 h-8 hover:bg-muted">
                              <MoreHorizontal className="h-4 w-4" />
                            </Button>
                          </div>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="pvs">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <CardTitle className="text-foreground">Persistent Volumes</CardTitle>
              <CardDescription>Cluster-wide storage resources</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="overflow-hidden rounded-xl border border-border/30">
                <Table>
                  <TableHeader className="bg-muted/80">
                    <TableRow className="border-border/30">
                      <TableHead className="font-semibold text-foreground">Name</TableHead>
                      <TableHead className="font-semibold text-foreground">Capacity</TableHead>
                      <TableHead className="font-semibold text-foreground">Access Modes</TableHead>
                      <TableHead className="font-semibold text-foreground">Reclaim Policy</TableHead>
                      <TableHead className="font-semibold text-foreground">Status</TableHead>
                      <TableHead className="font-semibold text-foreground">Claim</TableHead>
                      <TableHead className="font-semibold text-foreground">Storage Class</TableHead>
                      <TableHead className="font-semibold text-foreground">Age</TableHead>
                      <TableHead className="font-semibold text-foreground"></TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredPVs.map((pv) => (
                      <TableRow key={pv.name} className="border-border/30 hover:bg-muted/30 transition-colors">
                        <TableCell className="font-medium text-foreground">{pv.name}</TableCell>
                        <TableCell className="text-foreground">{pv.capacity}</TableCell>
                        <TableCell>
                          <Badge variant="outline">{pv.accessModes}</Badge>
                        </TableCell>
                        <TableCell className="text-foreground">{pv.reclaimPolicy}</TableCell>
                        <TableCell>{getStatusBadge(pv.status)}</TableCell>
                        <TableCell className="font-mono text-sm text-foreground">{pv.claim}</TableCell>
                        <TableCell className="text-foreground">{pv.storageClass}</TableCell>
                        <TableCell className="text-muted-foreground">{pv.age}</TableCell>
                        <TableCell>
                          <Button variant="ghost" size="sm" className="w-8 h-8 hover:bg-muted">
                            <MoreHorizontal className="h-4 w-4" />
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

        <TabsContent value="pvcs">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <CardTitle className="text-foreground">Persistent Volume Claims</CardTitle>
              <CardDescription>Storage requests by your applications</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="overflow-hidden rounded-xl border border-border/30">
                <Table>
                  <TableHeader className="bg-muted/80">
                    <TableRow className="border-border/30">
                      <TableHead className="font-semibold text-foreground">Name</TableHead>
                      <TableHead className="font-semibold text-foreground">Status</TableHead>
                      <TableHead className="font-semibold text-foreground">Volume</TableHead>
                      <TableHead className="font-semibold text-foreground">Capacity</TableHead>
                      <TableHead className="font-semibold text-foreground">Access Modes</TableHead>
                      <TableHead className="font-semibold text-foreground">Storage Class</TableHead>
                      <TableHead className="font-semibold text-foreground">Age</TableHead>
                      <TableHead className="font-semibold text-foreground"></TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredPVCs.map((pvc) => (
                      <TableRow key={pvc.name} className="border-border/30 hover:bg-muted/30 transition-colors">
                        <TableCell className="font-medium text-foreground">{pvc.name}</TableCell>
                        <TableCell>{getStatusBadge(pvc.status)}</TableCell>
                        <TableCell className="font-mono text-sm text-foreground">{pvc.volume}</TableCell>
                        <TableCell className="text-foreground">{pvc.capacity}</TableCell>
                        <TableCell>
                          <Badge variant="outline">{pvc.accessModes}</Badge>
                        </TableCell>
                        <TableCell className="text-foreground">{pvc.storageClass}</TableCell>
                        <TableCell className="text-muted-foreground">{pvc.age}</TableCell>
                        <TableCell>
                          <Button variant="ghost" size="sm" className="w-8 h-8 hover:bg-muted">
                            <MoreHorizontal className="h-4 w-4" />
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
      </Tabs>
    </div>
  );
}