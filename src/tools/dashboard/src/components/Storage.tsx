import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "./ui/table";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Search, MoreHorizontal, Plus, Database, HardDrive, Archive, Settings } from "lucide-react";

interface StorageProps {
  namespace: string;
}

export function Storage({ namespace }: StorageProps) {
  const [searchTerm, setSearchTerm] = useState("");

  // Mock data
  const persistentVolumes = [
    {
      name: "pv-database-01",
      capacity: "100Gi",
      accessModes: ["ReadWriteOnce"],
      reclaimPolicy: "Retain",
      status: "Bound",
      claim: "default/postgres-data",
      storageClass: "ssd",
      age: "15d"
    },
    {
      name: "pv-logs-01",
      capacity: "50Gi",
      accessModes: ["ReadWriteMany"],
      reclaimPolicy: "Delete",
      status: "Available",
      claim: "-",
      storageClass: "standard",
      age: "3d"
    },
    {
      name: "pv-cache-01",
      capacity: "20Gi",
      accessModes: ["ReadWriteOnce"],
      reclaimPolicy: "Delete",
      status: "Bound",
      claim: "default/redis-cache",
      storageClass: "fast-ssd",
      age: "7d"
    }
  ];

  const persistentVolumeClaims = [
    {
      name: "postgres-data",
      namespace: "default",
      status: "Bound",
      volume: "pv-database-01",
      capacity: "100Gi",
      accessModes: ["ReadWriteOnce"],
      storageClass: "ssd",
      age: "15d"
    },
    {
      name: "redis-cache",
      namespace: "default",
      status: "Bound",
      volume: "pv-cache-01",
      capacity: "20Gi",
      accessModes: ["ReadWriteOnce"],
      storageClass: "fast-ssd",
      age: "7d"
    },
    {
      name: "app-logs",
      namespace: "default",
      status: "Pending",
      volume: "-",
      capacity: "10Gi",
      accessModes: ["ReadWriteMany"],
      storageClass: "standard",
      age: "2h"
    }
  ];

  const storageClasses = [
    {
      name: "standard",
      provisioner: "kubernetes.io/aws-ebs",
      reclaimPolicy: "Delete",
      volumeBindingMode: "Immediate",
      allowVolumeExpansion: true,
      age: "30d"
    },
    {
      name: "ssd",
      provisioner: "kubernetes.io/aws-ebs",
      reclaimPolicy: "Retain",
      volumeBindingMode: "Immediate",
      allowVolumeExpansion: true,
      age: "30d"
    },
    {
      name: "fast-ssd",
      provisioner: "kubernetes.io/aws-ebs",
      reclaimPolicy: "Delete",
      volumeBindingMode: "WaitForFirstConsumer",
      allowVolumeExpansion: false,
      age: "30d"
    }
  ];

  const getStatusBadge = (status: string) => {
    switch (status) {
      case "Bound":
        return (
          <Badge className="bg-emerald-100 dark:bg-emerald-950 text-emerald-800 dark:text-emerald-200 border-emerald-200 dark:border-emerald-800">
            <div className="w-2 h-2 bg-emerald-500 rounded-full mr-1 animate-pulse"></div>
            Bound
          </Badge>
        );
      case "Available":
        return (
          <Badge className="bg-blue-100 dark:bg-blue-950 text-blue-800 dark:text-blue-200 border-blue-200 dark:border-blue-800">
            <div className="w-2 h-2 bg-blue-500 rounded-full mr-1"></div>
            Available
          </Badge>
        );
      case "Pending":
        return (
          <Badge className="bg-amber-100 dark:bg-amber-950 text-amber-800 dark:text-amber-200 border-amber-200 dark:border-amber-800">
            <div className="w-2 h-2 bg-amber-500 rounded-full mr-1"></div>
            Pending
          </Badge>
        );
      default:
        return <Badge variant="secondary">{status}</Badge>;
    }
  };

  const filteredPVs = persistentVolumes.filter(pv => 
    pv.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const filteredPVCs = persistentVolumeClaims.filter(pvc => 
    pvc.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const filteredSCs = storageClasses.filter(sc => 
    sc.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <div className="relative">
          <div className="flex items-center gap-4 mb-2">
            <div className="w-1 h-8 bg-gradient-to-b from-primary to-primary/80 rounded-full"></div>
            <h1 className="font-bold text-foreground">
              PULLPIRI Storage
            </h1>
          </div>
          <p className="text-muted-foreground ml-8">
            Manage persistent volumes, claims, and storage classes in <span className="font-semibold text-primary">"{namespace}"</span>
          </p>
        </div>
        <Button className="bg-primary hover:bg-primary/80 text-primary-foreground shadow-lg hover:shadow-xl transition-all gap-2">
          <Plus className="w-4 h-4" />
          Create PVC
        </Button>
      </div>

      <div className="flex items-center gap-4">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search storage resources..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="pl-10 h-12 bg-card/80 backdrop-blur-sm border-border/30 shadow-sm hover:shadow-md transition-all"
          />
        </div>
        <div className="flex items-center gap-3">
          <Badge className="bg-slate-50 dark:bg-slate-950 text-slate-700 dark:text-slate-300 border-slate-200 dark:border-slate-800 px-3 py-1">
            <Database className="w-3 h-3 mr-1" />
            {persistentVolumes.length} PVs
          </Badge>
          <Badge className="bg-slate-50 dark:bg-slate-950 text-slate-700 dark:text-slate-300 border-slate-200 dark:border-slate-800 px-3 py-1">
            <HardDrive className="w-3 h-3 mr-1" />
            {persistentVolumeClaims.length} PVCs
          </Badge>
        </div>
      </div>

      <Tabs defaultValue="pvcs" className="space-y-6">
        <TabsList className="bg-card/80 backdrop-blur-sm border border-border/30 shadow-lg">
          <TabsTrigger value="pvcs" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">
            Persistent Volume Claims
          </TabsTrigger>
          <TabsTrigger value="pvs" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">
            Persistent Volumes
          </TabsTrigger>
          <TabsTrigger value="storage-classes" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">
            Storage Classes
          </TabsTrigger>
        </TabsList>

        <TabsContent value="pvcs">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                  <HardDrive className="w-4 h-4 text-primary-foreground" />
                </div>
                <div>
                  <CardTitle className="text-foreground">Persistent Volume Claims</CardTitle>
                  <CardDescription>Storage requests from your applications</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="overflow-hidden rounded-xl border border-border/30">
                <Table>
                  <TableHeader className="bg-muted/80">
                    <TableRow className="border-border/30">
                      <TableHead className="font-semibold text-foreground">Name</TableHead>
                      <TableHead className="font-semibold text-foreground">Namespace</TableHead>
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
                        <TableCell className="text-muted-foreground">{pvc.namespace}</TableCell>
                        <TableCell>{getStatusBadge(pvc.status)}</TableCell>
                        <TableCell className="font-mono text-sm">
                          {pvc.volume !== "-" ? (
                            <Badge variant="outline" className="text-xs">
                              {pvc.volume}
                            </Badge>
                          ) : (
                            <span className="text-muted-foreground">-</span>
                          )}
                        </TableCell>
                        <TableCell className="font-mono text-sm">{pvc.capacity}</TableCell>
                        <TableCell>
                          <div className="flex gap-1 flex-wrap">
                            {pvc.accessModes.map((mode) => (
                              <Badge key={mode} variant="secondary" className="text-xs">
                                {mode}
                              </Badge>
                            ))}
                          </div>
                        </TableCell>
                        <TableCell>
                          <Badge variant="outline" className="text-xs">
                            {pvc.storageClass}
                          </Badge>
                        </TableCell>
                        <TableCell className="text-muted-foreground">{pvc.age}</TableCell>
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

        <TabsContent value="pvs">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                  <Database className="w-4 h-4 text-primary-foreground" />
                </div>
                <div>
                  <CardTitle className="text-foreground">Persistent Volumes</CardTitle>
                  <CardDescription>Cluster-wide storage resources</CardDescription>
                </div>
              </div>
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
                        <TableCell className="font-mono text-sm">{pv.capacity}</TableCell>
                        <TableCell>
                          <div className="flex gap-1 flex-wrap">
                            {pv.accessModes.map((mode) => (
                              <Badge key={mode} variant="secondary" className="text-xs">
                                {mode}
                              </Badge>
                            ))}
                          </div>
                        </TableCell>
                        <TableCell>
                          <Badge variant="outline" className="text-xs">
                            {pv.reclaimPolicy}
                          </Badge>
                        </TableCell>
                        <TableCell>{getStatusBadge(pv.status)}</TableCell>
                        <TableCell className="font-mono text-sm">
                          {pv.claim !== "-" ? (
                            <Badge variant="outline" className="text-xs">
                              {pv.claim}
                            </Badge>
                          ) : (
                            <span className="text-muted-foreground">-</span>
                          )}
                        </TableCell>
                        <TableCell>
                          <Badge variant="outline" className="text-xs">
                            {pv.storageClass}
                          </Badge>
                        </TableCell>
                        <TableCell className="text-muted-foreground">{pv.age}</TableCell>
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

        <TabsContent value="storage-classes">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                  <Settings className="w-4 h-4 text-primary-foreground" />
                </div>
                <div>
                  <CardTitle className="text-foreground">Storage Classes</CardTitle>
                  <CardDescription>Define storage provisioning parameters</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="overflow-hidden rounded-xl border border-border/30">
                <Table>
                  <TableHeader className="bg-muted/80">
                    <TableRow className="border-border/30">
                      <TableHead className="font-semibold text-foreground">Name</TableHead>
                      <TableHead className="font-semibold text-foreground">Provisioner</TableHead>
                      <TableHead className="font-semibold text-foreground">Reclaim Policy</TableHead>
                      <TableHead className="font-semibold text-foreground">Volume Binding Mode</TableHead>
                      <TableHead className="font-semibold text-foreground">Allow Volume Expansion</TableHead>
                      <TableHead className="font-semibold text-foreground">Age</TableHead>
                      <TableHead className="font-semibold text-foreground"></TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredSCs.map((sc) => (
                      <TableRow key={sc.name} className="border-border/30 hover:bg-muted/30 transition-colors">
                        <TableCell className="font-medium text-foreground">{sc.name}</TableCell>
                        <TableCell className="font-mono text-sm text-muted-foreground">{sc.provisioner}</TableCell>
                        <TableCell>
                          <Badge variant="outline" className="text-xs">
                            {sc.reclaimPolicy}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <Badge variant="secondary" className="text-xs">
                            {sc.volumeBindingMode}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <Badge className={sc.allowVolumeExpansion 
                            ? "bg-emerald-100 dark:bg-emerald-950 text-emerald-800 dark:text-emerald-200" 
                            : "bg-red-100 dark:bg-red-950 text-red-800 dark:text-red-200"
                          }>
                            {sc.allowVolumeExpansion ? "Yes" : "No"}
                          </Badge>
                        </TableCell>
                        <TableCell className="text-muted-foreground">{sc.age}</TableCell>
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
      </Tabs>
    </div>
  );
}