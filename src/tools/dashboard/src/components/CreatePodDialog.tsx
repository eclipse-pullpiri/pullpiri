import { useState } from "react";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogFooter } from "./ui/dialog";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Textarea } from "./ui/textarea";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { Badge } from "./ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { ScrollArea } from "./ui/scroll-area";
import { Separator } from "./ui/separator";
import { Alert, AlertDescription } from "./ui/alert";
import { Plus, X, Container, Settings, AlertTriangle, CheckCircle, Info, Cpu, MemoryStick } from "lucide-react";

interface CreatePodDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreatePod: (pod: any) => void;
}

interface ContainerConfig {
  name: string;
  image: string;
  ports: { containerPort: number; protocol: string }[];
  env: { name: string; value: string }[];
  resources: {
    requests: { cpu: string; memory: string };
    limits: { cpu: string; memory: string };
  };
}

export function CreatePodDialog({ open, onOpenChange, onCreatePod }: CreatePodDialogProps) {
  const [isCreating, setIsCreating] = useState(false);
  const [activeTab, setActiveTab] = useState("basic");
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});

  // Pod configuration state
  const [podConfig, setPodConfig] = useState({
    name: "",
    namespace: "default",
    labels: {} as Record<string, string>,
    nodeSelector: {} as Record<string, string>,
    restartPolicy: "Always",
    dnsPolicy: "ClusterFirst"
  });

  // Container configuration state
  const [containers, setContainers] = useState<ContainerConfig[]>([
    {
      name: "",
      image: "",
      ports: [],
      env: [],
      resources: {
        requests: { cpu: "100m", memory: "128Mi" },
        limits: { cpu: "500m", memory: "512Mi" }
      }
    }
  ]);

  // Label management
  const [newLabel, setNewLabel] = useState({ key: "", value: "" });

  // Environment variable management
  const [newEnvVar, setNewEnvVar] = useState({ name: "", value: "" });

  // Port management
  const [newPort, setNewPort] = useState({ containerPort: "", protocol: "TCP" });

  // Validation
  const validateConfiguration = () => {
    const errors: Record<string, string> = {};

    if (!podConfig.name.trim()) {
      errors.podName = "Pod name is required";
    } else if (!/^[a-z0-9]([-a-z0-9]*[a-z0-9])?$/.test(podConfig.name)) {
      errors.podName = "Pod name must be lowercase alphanumeric with hyphens";
    }

    containers.forEach((container, index) => {
      if (!container.name.trim()) {
        errors[`containerName${index}`] = "Container name is required";
      }
      if (!container.image.trim()) {
        errors[`containerImage${index}`] = "Container image is required";
      }
    });

    setValidationErrors(errors);
    return Object.keys(errors).length === 0;
  };

  // Handle pod creation
  const handleCreatePod = async () => {
    if (!validateConfiguration()) {
      return;
    }

    setIsCreating(true);

    try {
      // Simulate API call
      await new Promise(resolve => setTimeout(resolve, 2000));

      // Create pod object
      const newPod = {
        name: podConfig.name,
        image: containers[0].image,
        labels: podConfig.labels,
        node: "worker-node-01", // Mock node assignment
        status: "Pending",
        cpuUsage: "0m",
        memoryUsage: "0Mi",
        age: "0s",
        ready: "0/1",
        restarts: 0,
        ip: "Pending"
      };

      onCreatePod(newPod);
      resetForm();
      onOpenChange(false);
      
      // Success notification would go here
      console.log("✅ Pod created successfully:", newPod);
    } catch (error) {
      console.error("❌ Failed to create pod:", error);
    } finally {
      setIsCreating(false);
    }
  };

  // Reset form
  const resetForm = () => {
    setPodConfig({
      name: "",
      namespace: "default",
      labels: {},
      nodeSelector: {},
      restartPolicy: "Always",
      dnsPolicy: "ClusterFirst"
    });
    setContainers([
      {
        name: "",
        image: "",
        ports: [],
        env: [],
        resources: {
          requests: { cpu: "100m", memory: "128Mi" },
          limits: { cpu: "500m", memory: "512Mi" }
        }
      }
    ]);
    setValidationErrors({});
    setActiveTab("basic");
  };

  // Add label
  const addLabel = () => {
    if (newLabel.key.trim() && newLabel.value.trim()) {
      setPodConfig(prev => ({
        ...prev,
        labels: { ...prev.labels, [newLabel.key.trim()]: newLabel.value.trim() }
      }));
      setNewLabel({ key: "", value: "" });
    }
  };

  // Remove label
  const removeLabel = (key: string) => {
    setPodConfig(prev => {
      const newLabels = { ...prev.labels };
      delete newLabels[key];
      return { ...prev, labels: newLabels };
    });
  };

  // Add environment variable
  const addEnvVar = (containerIndex: number) => {
    if (newEnvVar.name.trim() && newEnvVar.value.trim()) {
      setContainers(prev => prev.map((container, index) => {
        if (index === containerIndex) {
          return {
            ...container,
            env: [...container.env, { name: newEnvVar.name.trim(), value: newEnvVar.value.trim() }]
          };
        }
        return container;
      }));
      setNewEnvVar({ name: "", value: "" });
    }
  };

  // Add port
  const addPort = (containerIndex: number) => {
    const port = parseInt(newPort.containerPort);
    if (port && port > 0 && port < 65536) {
      setContainers(prev => prev.map((container, index) => {
        if (index === containerIndex) {
          return {
            ...container,
            ports: [...container.ports, { containerPort: port, protocol: newPort.protocol }]
          };
        }
        return container;
      }));
      setNewPort({ containerPort: "", protocol: "TCP" });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[85vw] max-w-4xl h-[80vh] max-h-[900px] flex flex-col bg-card/95 backdrop-blur-sm">
        <DialogHeader className="flex-shrink-0">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-primary rounded-lg flex items-center justify-center">
              <Container className="w-5 h-5 text-primary-foreground" />
            </div>
            <div>
              <DialogTitle className="text-foreground">Create New Pod</DialogTitle>
              <DialogDescription>
                Configure and deploy a new pod to your Kubernetes cluster
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <div className="flex-1 min-h-0">
          <Tabs value={activeTab} onValueChange={setActiveTab} className="h-full flex flex-col">
            <TabsList className="bg-muted/50 border border-border/30 self-start">
              <TabsTrigger value="basic" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">
                <Container className="w-4 h-4 mr-2" />
                Basic Info
              </TabsTrigger>
              <TabsTrigger value="containers" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">
                <Settings className="w-4 h-4 mr-2" />
                Containers
              </TabsTrigger>
            </TabsList>

            <div className="flex-1 min-h-0 mt-4">
              <ScrollArea className="h-full">
                <TabsContent value="basic" className="space-y-6 mt-0">
                  <Card>
                    <CardHeader>
                      <CardTitle className="flex items-center gap-2">
                        <Info className="w-4 h-4" />
                        Pod Metadata
                      </CardTitle>
                      <CardDescription>Basic information about your pod</CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-4">
                      <div className="grid grid-cols-2 gap-4">
                        <div className="space-y-2">
                          <Label htmlFor="podName">Pod Name *</Label>
                          <Input
                            id="podName"
                            placeholder="my-app-pod"
                            value={podConfig.name}
                            onChange={(e) => setPodConfig(prev => ({ ...prev, name: e.target.value }))}
                            className={validationErrors.podName ? "border-destructive" : ""}
                          />
                          {validationErrors.podName && (
                            <Alert className="border-destructive/50">
                              <AlertTriangle className="h-4 w-4" />
                              <AlertDescription className="text-destructive">
                                {validationErrors.podName}
                              </AlertDescription>
                            </Alert>
                          )}
                        </div>
                        <div className="space-y-2">
                          <Label htmlFor="namespace">Namespace</Label>
                          <Select value={podConfig.namespace} onValueChange={(value) => setPodConfig(prev => ({ ...prev, namespace: value }))}>
                            <SelectTrigger>
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="default">default</SelectItem>
                              <SelectItem value="kube-system">kube-system</SelectItem>
                              <SelectItem value="monitoring">monitoring</SelectItem>
                              <SelectItem value="logging">logging</SelectItem>
                            </SelectContent>
                          </Select>
                        </div>
                      </div>

                      <div className="grid grid-cols-2 gap-4">
                        <div className="space-y-2">
                          <Label>Restart Policy</Label>
                          <Select value={podConfig.restartPolicy} onValueChange={(value) => setPodConfig(prev => ({ ...prev, restartPolicy: value }))}>
                            <SelectTrigger>
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="Always">Always</SelectItem>
                              <SelectItem value="OnFailure">OnFailure</SelectItem>
                              <SelectItem value="Never">Never</SelectItem>
                            </SelectContent>
                          </Select>
                        </div>
                        <div className="space-y-2">
                          <Label>DNS Policy</Label>
                          <Select value={podConfig.dnsPolicy} onValueChange={(value) => setPodConfig(prev => ({ ...prev, dnsPolicy: value }))}>
                            <SelectTrigger>
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="ClusterFirst">ClusterFirst</SelectItem>
                              <SelectItem value="Default">Default</SelectItem>
                              <SelectItem value="ClusterFirstWithHostNet">ClusterFirstWithHostNet</SelectItem>
                            </SelectContent>
                          </Select>
                        </div>
                      </div>

                      <Separator />

                      <div className="space-y-4">
                        <Label>Labels</Label>
                        <div className="flex gap-2">
                          <Input
                            placeholder="Key"
                            value={newLabel.key}
                            onChange={(e) => setNewLabel(prev => ({ ...prev, key: e.target.value }))}
                            className="flex-1"
                          />
                          <Input
                            placeholder="Value"
                            value={newLabel.value}
                            onChange={(e) => setNewLabel(prev => ({ ...prev, value: e.target.value }))}
                            className="flex-1"
                          />
                          <Button onClick={addLabel} variant="outline" size="sm">
                            <Plus className="w-4 h-4" />
                          </Button>
                        </div>
                        <div className="flex gap-2 flex-wrap">
                          {Object.entries(podConfig.labels).map(([key, value]) => (
                            <Badge key={key} variant="secondary" className="gap-1">
                              {key}={value}
                              <button onClick={() => removeLabel(key)} className="ml-1">
                                <X className="w-3 h-3" />
                              </button>
                            </Badge>
                          ))}
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                </TabsContent>

                <TabsContent value="containers" className="space-y-6 mt-0">
                  {containers.map((container, index) => (
                    <Card key={index}>
                      <CardHeader>
                        <CardTitle className="flex items-center gap-2">
                          <Container className="w-4 h-4" />
                          Container {index + 1}
                        </CardTitle>
                        <CardDescription>Configure container image and settings</CardDescription>
                      </CardHeader>
                      <CardContent className="space-y-4">
                        <div className="grid grid-cols-2 gap-4">
                          <div className="space-y-2">
                            <Label htmlFor={`containerName${index}`}>Container Name *</Label>
                            <Input
                              id={`containerName${index}`}
                              placeholder="nginx"
                              value={container.name}
                              onChange={(e) => setContainers(prev => prev.map((c, i) => i === index ? { ...c, name: e.target.value } : c))}
                              className={validationErrors[`containerName${index}`] ? "border-destructive" : ""}
                            />
                            {validationErrors[`containerName${index}`] && (
                              <p className="text-sm text-destructive">{validationErrors[`containerName${index}`]}</p>
                            )}
                          </div>
                          <div className="space-y-2">
                            <Label htmlFor={`containerImage${index}`}>Container Image *</Label>
                            <Input
                              id={`containerImage${index}`}
                              placeholder="nginx:1.21"
                              value={container.image}
                              onChange={(e) => setContainers(prev => prev.map((c, i) => i === index ? { ...c, image: e.target.value } : c))}
                              className={validationErrors[`containerImage${index}`] ? "border-destructive" : ""}
                            />
                            {validationErrors[`containerImage${index}`] && (
                              <p className="text-sm text-destructive">{validationErrors[`containerImage${index}`]}</p>
                            )}
                          </div>
                        </div>

                        <Separator />

                        <div className="space-y-4">
                          <Label>Resource Limits</Label>
                          <div className="grid grid-cols-4 gap-4">
                            <div className="space-y-2">
                              <Label className="text-xs text-muted-foreground flex items-center gap-1">
                                <Cpu className="w-3 h-3" />
                                CPU Request
                              </Label>
                              <Input
                                placeholder="100m"
                                value={container.resources.requests.cpu}
                                onChange={(e) => setContainers(prev => prev.map((c, i) => 
                                  i === index ? { 
                                    ...c, 
                                    resources: { ...c.resources, requests: { ...c.resources.requests, cpu: e.target.value } }
                                  } : c
                                ))}
                              />
                            </div>
                            <div className="space-y-2">
                              <Label className="text-xs text-muted-foreground flex items-center gap-1">
                                <MemoryStick className="w-3 h-3" />
                                Memory Request
                              </Label>
                              <Input
                                placeholder="128Mi"
                                value={container.resources.requests.memory}
                                onChange={(e) => setContainers(prev => prev.map((c, i) => 
                                  i === index ? { 
                                    ...c, 
                                    resources: { ...c.resources, requests: { ...c.resources.requests, memory: e.target.value } }
                                  } : c
                                ))}
                              />
                            </div>
                            <div className="space-y-2">
                              <Label className="text-xs text-muted-foreground flex items-center gap-1">
                                <Cpu className="w-3 h-3" />
                                CPU Limit
                              </Label>
                              <Input
                                placeholder="500m"
                                value={container.resources.limits.cpu}
                                onChange={(e) => setContainers(prev => prev.map((c, i) => 
                                  i === index ? { 
                                    ...c, 
                                    resources: { ...c.resources, limits: { ...c.resources.limits, cpu: e.target.value } }
                                  } : c
                                ))}
                              />
                            </div>
                            <div className="space-y-2">
                              <Label className="text-xs text-muted-foreground flex items-center gap-1">
                                <MemoryStick className="w-3 h-3" />
                                Memory Limit
                              </Label>
                              <Input
                                placeholder="512Mi"
                                value={container.resources.limits.memory}
                                onChange={(e) => setContainers(prev => prev.map((c, i) => 
                                  i === index ? { 
                                    ...c, 
                                    resources: { ...c.resources, limits: { ...c.resources.limits, memory: e.target.value } }
                                  } : c
                                ))}
                              />
                            </div>
                          </div>
                        </div>

                        <Separator />

                        <div className="space-y-4">
                          <Label>Ports</Label>
                          <div className="flex gap-2">
                            <Input
                              placeholder="Port"
                              type="number"
                              value={newPort.containerPort}
                              onChange={(e) => setNewPort(prev => ({ ...prev, containerPort: e.target.value }))}
                              className="w-24"
                            />
                            <Select value={newPort.protocol} onValueChange={(value) => setNewPort(prev => ({ ...prev, protocol: value }))}>
                              <SelectTrigger className="w-24">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                <SelectItem value="TCP">TCP</SelectItem>
                                <SelectItem value="UDP">UDP</SelectItem>
                              </SelectContent>
                            </Select>
                            <Button onClick={() => addPort(index)} variant="outline" size="sm">
                              <Plus className="w-4 h-4" />
                            </Button>
                          </div>
                          <div className="flex gap-2 flex-wrap">
                            {container.ports.map((port, portIndex) => (
                              <Badge key={portIndex} variant="outline">
                                {port.containerPort}/{port.protocol}
                              </Badge>
                            ))}
                          </div>
                        </div>

                        <Separator />

                        <div className="space-y-4">
                          <Label>Environment Variables</Label>
                          <div className="flex gap-2">
                            <Input
                              placeholder="Name"
                              value={newEnvVar.name}
                              onChange={(e) => setNewEnvVar(prev => ({ ...prev, name: e.target.value }))}
                              className="flex-1"
                            />
                            <Input
                              placeholder="Value"
                              value={newEnvVar.value}
                              onChange={(e) => setNewEnvVar(prev => ({ ...prev, value: e.target.value }))}
                              className="flex-1"
                            />
                            <Button onClick={() => addEnvVar(index)} variant="outline" size="sm">
                              <Plus className="w-4 h-4" />
                            </Button>
                          </div>
                          <div className="flex gap-2 flex-wrap">
                            {container.env.map((env, envIndex) => (
                              <Badge key={envIndex} variant="secondary">
                                {env.name}={env.value}
                              </Badge>
                            ))}
                          </div>
                        </div>
                      </CardContent>
                    </Card>
                  ))}
                </TabsContent>
              </ScrollArea>
            </div>
          </Tabs>
        </div>

        <DialogFooter className="flex-shrink-0 border-t border-border/20 pt-4">
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isCreating}>
            Cancel
          </Button>
          <Button onClick={handleCreatePod} disabled={isCreating} className="min-w-[120px]">
            {isCreating ? (
              <>
                <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin mr-2" />
                Creating...
              </>
            ) : (
              <>
                <CheckCircle className="w-4 h-4 mr-2" />
                Create Pod
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}