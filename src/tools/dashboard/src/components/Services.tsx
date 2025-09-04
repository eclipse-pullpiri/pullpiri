import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "./ui/table";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Search, MoreHorizontal, ExternalLink, Copy } from "lucide-react";

interface ServicesProps {
  namespace: string;
}

export function Services({ namespace }: ServicesProps) {
  const [searchTerm, setSearchTerm] = useState("");

  // Mock data
  const services = [
    {
      name: "frontend-service",
      type: "ClusterIP",
      clusterIP: "10.96.0.15",
      externalIP: "N/A",
      ports: "80/TCP",
      age: "2d",
      selector: "app=frontend"
    },
    {
      name: "backend-service", 
      type: "ClusterIP",
      clusterIP: "10.96.1.22",
      externalIP: "N/A", 
      ports: "8080/TCP",
      age: "5d",
      selector: "app=backend"
    },
    {
      name: "redis-service",
      type: "ClusterIP",
      clusterIP: "10.96.2.8",
      externalIP: "N/A",
      ports: "6379/TCP", 
      age: "1d",
      selector: "app=redis"
    },
    {
      name: "nginx-ingress",
      type: "LoadBalancer",
      clusterIP: "10.96.3.45",
      externalIP: "203.0.113.15",
      ports: "80:30080/TCP,443:30443/TCP",
      age: "7d",
      selector: "app=nginx-ingress"
    }
  ];

  const ingresses = [
    {
      name: "frontend-ingress",
      className: "nginx",
      hosts: "frontend.example.com",
      address: "203.0.113.15",
      ports: "80, 443",
      age: "2d"
    },
    {
      name: "api-ingress",
      className: "nginx", 
      hosts: "api.example.com",
      address: "203.0.113.15",
      ports: "80, 443",
      age: "5d"
    },
    {
      name: "monitoring-ingress",
      className: "nginx",
      hosts: "monitoring.example.com", 
      address: "203.0.113.15",
      ports: "80, 443",
      age: "7d"
    }
  ];

  const endpoints = [
    {
      name: "frontend-service",
      endpoints: "10.244.1.15:80,10.244.2.18:80,10.244.3.12:80",
      age: "2d"
    },
    {
      name: "backend-service",
      endpoints: "10.244.1.22:8080,10.244.2.31:8080",
      age: "5d"
    },
    {
      name: "redis-service", 
      endpoints: "10.244.3.9:6379",
      age: "1d"
    }
  ];

  const getServiceTypeBadge = (type: string) => {
    switch (type) {
      case "ClusterIP":
        return <Badge variant="secondary">ClusterIP</Badge>;
      case "NodePort":
        return <Badge className="bg-blue-100 dark:bg-blue-950 text-blue-800 dark:text-blue-200 hover:bg-blue-100 dark:hover:bg-blue-950">NodePort</Badge>;
      case "LoadBalancer":
        return <Badge className="bg-green-100 dark:bg-green-950 text-green-800 dark:text-green-200 hover:bg-green-100 dark:hover:bg-green-950">LoadBalancer</Badge>;
      case "ExternalName":
        return <Badge className="bg-purple-100 dark:bg-purple-950 text-purple-800 dark:text-purple-200 hover:bg-purple-100 dark:hover:bg-purple-950">ExternalName</Badge>;
      default:
        return <Badge variant="secondary">{type}</Badge>;
    }
  };

  const filteredServices = services.filter(service => 
    service.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const filteredIngresses = ingresses.filter(ingress => 
    ingress.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const filteredEndpoints = endpoints.filter(endpoint => 
    endpoint.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="relative">
          <div className="flex items-center gap-4 mb-2">
            <div className="w-1 h-8 bg-gradient-to-b from-primary to-primary/80 rounded-full"></div>
            <h1 className="font-bold text-foreground">
              PULLPIRI Services & Networking
            </h1>
          </div>
          <p className="text-muted-foreground ml-8">
            Manage services, ingresses, and network policies in <span className="font-semibold text-primary">"{namespace}"</span>
          </p>
        </div>
        <Button className="bg-primary hover:bg-primary/80 text-primary-foreground shadow-lg hover:shadow-xl transition-all">
          Create Service
        </Button>
      </div>

      <div className="flex items-center gap-4">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search services..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="pl-10 h-12 bg-card/80 backdrop-blur-sm border-border/30 shadow-sm hover:shadow-md transition-all"
          />
        </div>
      </div>

      <Tabs defaultValue="services" className="space-y-6">
        <TabsList className="bg-card/80 backdrop-blur-sm border border-border/30 shadow-lg">
          <TabsTrigger value="services" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">Services</TabsTrigger>
          <TabsTrigger value="ingresses" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">Ingresses</TabsTrigger>
          <TabsTrigger value="endpoints" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">Endpoints</TabsTrigger>
          <TabsTrigger value="networkpolicies" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">Network Policies</TabsTrigger>
        </TabsList>

        <TabsContent value="services">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                  <div className="w-4 h-4 bg-primary-foreground rounded-full" />
                </div>
                <div>
                  <CardTitle className="text-foreground">Services</CardTitle>
                  <CardDescription>Kubernetes services expose your applications</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="overflow-hidden rounded-xl border border-border/30">
                <Table>
                  <TableHeader className="bg-muted/80">
                    <TableRow className="border-border/30">
                      <TableHead className="font-semibold text-foreground">Name</TableHead>
                      <TableHead className="font-semibold text-foreground">Type</TableHead>
                      <TableHead className="font-semibold text-foreground">Cluster-IP</TableHead>
                      <TableHead className="font-semibold text-foreground">External-IP</TableHead>
                      <TableHead className="font-semibold text-foreground">Ports</TableHead>
                      <TableHead className="font-semibold text-foreground">Age</TableHead>
                      <TableHead className="font-semibold text-foreground">Selector</TableHead>
                      <TableHead className="font-semibold text-foreground"></TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredServices.map((service) => (
                      <TableRow key={service.name} className="border-border/30 hover:bg-muted/30 transition-colors">
                        <TableCell className="font-medium text-foreground">{service.name}</TableCell>
                        <TableCell>{getServiceTypeBadge(service.type)}</TableCell>
                        <TableCell className="font-mono text-sm text-foreground">{service.clusterIP}</TableCell>
                        <TableCell className="font-mono text-sm text-foreground">{service.externalIP}</TableCell>
                        <TableCell className="font-mono text-sm text-foreground">{service.ports}</TableCell>
                        <TableCell className="text-muted-foreground">{service.age}</TableCell>
                        <TableCell className="text-sm text-muted-foreground">{service.selector}</TableCell>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <Button variant="ghost" size="sm" className="w-8 h-8 hover:bg-muted">
                              <Copy className="h-4 w-4" />
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

        <TabsContent value="ingresses">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                  <ExternalLink className="w-4 h-4 text-primary-foreground" />
                </div>
                <div>
                  <CardTitle className="text-foreground">Ingresses</CardTitle>
                  <CardDescription>HTTP and HTTPS routing to your services</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="overflow-hidden rounded-xl border border-border/30">
                <Table>
                  <TableHeader className="bg-muted/80">
                    <TableRow className="border-border/30">
                      <TableHead className="font-semibold text-foreground">Name</TableHead>
                      <TableHead className="font-semibold text-foreground">Class</TableHead>
                      <TableHead className="font-semibold text-foreground">Hosts</TableHead>
                      <TableHead className="font-semibold text-foreground">Address</TableHead>
                      <TableHead className="font-semibold text-foreground">Ports</TableHead>
                      <TableHead className="font-semibold text-foreground">Age</TableHead>
                      <TableHead className="font-semibold text-foreground"></TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredIngresses.map((ingress) => (
                      <TableRow key={ingress.name} className="border-border/30 hover:bg-muted/30 transition-colors">
                        <TableCell className="font-medium text-foreground">{ingress.name}</TableCell>
                        <TableCell>
                          <Badge variant="outline">{ingress.className}</Badge>
                        </TableCell>
                        <TableCell className="font-mono text-sm text-foreground">{ingress.hosts}</TableCell>
                        <TableCell className="font-mono text-sm text-foreground">{ingress.address}</TableCell>
                        <TableCell className="text-foreground">{ingress.ports}</TableCell>
                        <TableCell className="text-muted-foreground">{ingress.age}</TableCell>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <Button variant="ghost" size="sm" className="w-8 h-8 hover:bg-muted">
                              <ExternalLink className="h-4 w-4" />
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

        <TabsContent value="endpoints">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <CardTitle className="text-foreground">Endpoints</CardTitle>
              <CardDescription>Network endpoints for your services</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="overflow-hidden rounded-xl border border-border/30">
                <Table>
                  <TableHeader className="bg-muted/80">
                    <TableRow className="border-border/30">
                      <TableHead className="font-semibold text-foreground">Name</TableHead>
                      <TableHead className="font-semibold text-foreground">Endpoints</TableHead>
                      <TableHead className="font-semibold text-foreground">Age</TableHead>
                      <TableHead className="font-semibold text-foreground"></TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredEndpoints.map((endpoint) => (
                      <TableRow key={endpoint.name} className="border-border/30 hover:bg-muted/30 transition-colors">
                        <TableCell className="font-medium text-foreground">{endpoint.name}</TableCell>
                        <TableCell className="font-mono text-sm max-w-md truncate text-foreground" title={endpoint.endpoints}>
                          {endpoint.endpoints}
                        </TableCell>
                        <TableCell className="text-muted-foreground">{endpoint.age}</TableCell>
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

        <TabsContent value="networkpolicies">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <CardTitle className="text-foreground">Network Policies</CardTitle>
              <CardDescription>Configure network access rules for your pods</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="text-center py-16">
                <div className="w-16 h-16 bg-gradient-to-r from-muted to-muted/80 rounded-2xl flex items-center justify-center mx-auto mb-4">
                  <div className="w-8 h-8 text-muted-foreground" />
                </div>
                <h3 className="font-semibold text-foreground mb-2">Network Policies View</h3>
                <p className="text-muted-foreground">Implementation coming soon</p>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}