import { useState } from "react";
import { createPortal } from "react-dom";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "./ui/table";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
//import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from "./ui/alert-dialog";
import { Progress } from "./ui/progress";
//import { ChartContainer, ChartTooltip, ChartTooltipContent } from "./ui/chart";
import { Search, MoreHorizontal, Play/*, Pause*/, RotateCcw, Plus, Box, Activity, AlertCircle, Cpu, MemoryStick, FileText, Terminal, Edit, Trash2, Server, Network, TrendingUp, Zap, Clock/*, Users*/ } from "lucide-react";
import { LogsDialog } from "./LogsDialog";
import { TerminalView } from "./TerminalView";
import { YamlEditor } from "./YamlEditor";
import { CreatePodDialog } from "./CreatePodDialog";
import { PieChart, Pie, Cell, ResponsiveContainer,/* BarChart, Bar, XAxis, YAxis, CartesianGrid, Legend, LineChart, Line, Area, AreaChart, */Tooltip } from 'recharts';
import { useClusterHealth } from "./ui/use-cluster-health";

// Pod interface
export interface Pod {
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
}

// Mock data
const deployments = [
  {
    name: "frontend-app",
    namespace: "default",
    ready: "3/3",
    upToDate: 3,
    available: 3,
    age: "2d",
    status: "Running"
  },
  {
    name: "backend-api",
    namespace: "default", 
    ready: "2/2",
    upToDate: 2,
    available: 2,
    age: "5d",
    status: "Running"
  },
  {
    name: "redis-cache",
    namespace: "default",
    ready: "1/1",
    upToDate: 1,
    available: 1,
    age: "1d",
    status: "Running"
  },
  {
    name: "database-migration",
    namespace: "default",
    ready: "0/1",
    upToDate: 0,
    available: 0,
    age: "30m",
    status: "Pending"
  }
];

interface WorkloadsProps {
  namespace: string;
  onPodClick?: (podName: string) => void;
  pods: Pod[];
  setPods: React.Dispatch<React.SetStateAction<Pod[]>>;
}

export function Workloads({ namespace, onPodClick, pods, setPods }: WorkloadsProps) {
  const [searchTerm, setSearchTerm] = useState("");
  const [openMenus, setOpenMenus] = useState<Record<string, boolean>>({});
  const [menuPosition, setMenuPosition] = useState<{ top: number; right: number } | null>(null);
  
  // Dialog states
  const [logsDialog, setLogsDialog] = useState<{ open: boolean; podName: string }>({ 
    open: false, 
    podName: "" 
  });
  const [terminalView, setTerminalView] = useState<{ open: boolean; podName: string }>({ 
    open: false, 
    podName: "" 
  });
  const [yamlEditor, setYamlEditor] = useState<{ open: boolean; podName: string }>({ 
    open: false, 
    podName: "" 
  });
  
  // Delete confirmation dialog state
  const [deleteDialog, setDeleteDialog] = useState<{ open: boolean; podName: string }>({
    open: false,
    podName: ""
  });

  // Create Pod dialog state
  const [createPodDialog, setCreatePodDialog] = useState(false);

  // Toggle menu for specific pod
  const toggleMenu = (podName: string, e: React.MouseEvent) => {
    e.stopPropagation();
    console.log('🔍 Debug: Menu toggle clicked for pod:', podName);
    
    const rect = e.currentTarget.getBoundingClientRect();
    setMenuPosition({
      top: rect.bottom + window.scrollY,
      right: window.innerWidth - rect.right
    });
    
    setOpenMenus(prev => ({
      ...prev,
      [podName]: !prev[podName]
    }));
  };

  // Close all menus
  const closeAllMenus = () => {
    setOpenMenus({});
  };

  // Handle pod actions
  const handlePodAction = (action: string, podName: string) => {
    console.log(`🔧 Pod Action Triggered: ${action} for ${podName}`);
    switch (action) {
      case 'logs':
        console.log(`Opening logs for ${podName}`);
        setLogsDialog({ open: true, podName });
        break;
      case 'exec':
        console.log(`Executing shell for ${podName}`);
        setTerminalView({ open: true, podName });
        break;
      case 'edit':
        console.log(`Editing ${podName}`);
        setYamlEditor({ open: true, podName });
        break;
      case 'delete':
        console.log(`Requesting delete confirmation for ${podName}`);
        setDeleteDialog({ open: true, podName });
        break;
    }
  };

  // Handle actual pod deletion
  const handleConfirmDelete = () => {
    const podName = deleteDialog.podName;
    console.log(`✅ Deleting ${podName}`);
    
    // Actually delete the pod from the list
    setPods(prevPods => prevPods.filter(pod => pod.name !== podName));
    
    // Close the dialog
    setDeleteDialog({ open: false, podName: "" });
    
    console.log(`✅ Pod "${podName}" deleted successfully!`);
  };

  // Calculate cluster metrics
  //const runningPods = pods.filter(pod => pod.status === "Running").length;
  //const pendingPods = pods.filter(pod => pod.status === "Pending").length;  
  //const failedPods = pods.filter(pod => pod.status === "Failed").length;

  // Nodes data - move this before cluster health calculation
  /*const nodesData = [
    { name: 'worker-node-1', pods: 2, status: 'Ready', cpu: '2.4/4', memory: '3.2/8' },
    { name: 'worker-node-2', pods: 2, status: 'Ready', cpu: '1.8/4', memory: '2.1/8' },
    { name: 'worker-node-3', pods: 1, status: 'Ready', cpu: '0.8/4', memory: '1.5/8' }
  ];
*/
  // Calculate cluster health based on actual data
  //const totalPods = pods.length;
  //const runningPodPercentage = totalPods > 0 ? (runningPods / totalPods) * 100 : 100;
  //const healthyNodeCount = nodesData.filter(node => node.status === 'Ready').length;
  //const totalNodeCount = nodesData.length;
  //const nodeHealthPercentage = totalNodeCount > 0 ? (healthyNodeCount / totalNodeCount) * 100 : 100;
  
  // Determine cluster health status
  /*const getClusterHealth = () => {
    // Critical: Failed pods > 20% or any nodes down
    if (failedPods > totalPods * 0.2 || nodeHealthPercentage < 100) {
      return {
        status: "Critical",
        color: "text-red-600 dark:text-red-400",
        bgColor: "bg-red-500",
        dotColor: "bg-red-500",
        borderColor: "border-red-200/20 dark:border-red-800/20",
        bgGradient: "from-red-500/10 to-red-600/10"
      };
    }
    // Warning: Pending pods > 10% or running pods < 90%
    else if (pendingPods > totalPods * 0.1 || runningPodPercentage < 90) {
      return {
        status: "Warning",
        color: "text-amber-600 dark:text-amber-400",
        bgColor: "bg-amber-500",
        dotColor: "bg-amber-500",
        borderColor: "border-amber-200/20 dark:border-amber-800/20",
        bgGradient: "from-amber-500/10 to-amber-600/10"
      };
    }
    // Healthy: All systems operational
    else {
      return {
        status: "Healthy",
        color: "text-emerald-600 dark:text-emerald-400",
        bgColor: "bg-emerald-500",
        dotColor: "bg-emerald-500",
        borderColor: "border-emerald-200/20 dark:border-emerald-800/20",
        bgGradient: "from-emerald-500/10 to-emerald-600/10"
      };
    }
  };
*/
  // Use the shared cluster health hook
  const clusterHealth = useClusterHealth(pods);

  // Pod status data for chart
  const podStatusData = [
    { name: 'Running', value: clusterHealth.runningPods, color: '#10b981' },
    { name: 'Pending', value: clusterHealth.pendingPods, color: '#f59e0b' },
    { name: 'Failed', value: clusterHealth.failedPods, color: '#ef4444' }
  ].filter(item => item.value > 0);

  // Resource usage data
  const resourceData = [
    { name: 'CPU Usage', value: 68, max: 100, color: '#3b82f6' },
    { name: 'Memory Usage', value: 84, max: 100, color: '#8b5cf6' },
    { name: 'Storage Usage', value: 45, max: 100, color: '#06b6d4' },
  ];

  // Recent events
  const recentEvents = [
    { type: 'Created', resource: 'Pod', name: 'frontend-app-7d4b8c9f8d-xyz12', time: '2m ago', status: 'success' },
    { type: 'Scheduled', resource: 'Pod', name: 'backend-api-5f6a7b8c9d-def56', time: '5m ago', status: 'success' },
    { type: 'Failed', resource: 'Pod', name: 'database-migration-1a2b3c4d5e-jkl90', time: '30m ago', status: 'error' },
  ];

  // Helper function to get status badge
  const getStatusBadge = (status: string) => {
    const statusConfig = {
      'Running': { 
        variant: 'default' as const, 
        className: 'bg-emerald-100 dark:bg-emerald-950 text-emerald-800 dark:text-emerald-200 border-emerald-200 dark:border-emerald-800' 
      },
      'Pending': { 
        variant: 'secondary' as const, 
        className: 'bg-amber-100 dark:bg-amber-950 text-amber-800 dark:text-amber-200 border-amber-200 dark:border-amber-800' 
      },
      'Failed': { 
        variant: 'destructive' as const, 
        className: 'bg-red-100 dark:bg-red-950 text-red-800 dark:text-red-200 border-red-200 dark:border-red-800' 
      },
      'Succeeded': { 
        variant: 'default' as const, 
        className: 'bg-emerald-100 dark:bg-emerald-950 text-emerald-800 dark:text-emerald-200 border-emerald-200 dark:border-emerald-800' 
      }
    };

    const config = statusConfig[status as keyof typeof statusConfig] || statusConfig['Pending'];
    
    return (
      <Badge variant={config.variant} className={config.className}>
        {status}
      </Badge>
    );
  };

  const filteredDeployments = deployments.filter(dep => 
    dep.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const filteredPods = pods.filter(pod => 
    pod.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  return (
    <div className="space-y-8" onClick={closeAllMenus}>
      <div className="flex items-center justify-between">
        <div className="relative">
          <div className="flex items-center gap-4 mb-2">
            <div className="w-1 h-8 bg-gradient-to-b from-primary to-primary/80 rounded-full"></div>
            <h1 className="font-bold text-foreground">
              PULLPIRI Workloads
            </h1>
          </div>
          <p className="text-muted-foreground ml-8">
            Manage deployments, pods, and other workloads in <span className="font-semibold text-primary">"{namespace}"</span>
          </p>
        </div>
      </div>

      {/* Cluster Overview Dashboard */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        {/* Cluster Status */}
        <Card className={`bg-gradient-to-r ${clusterHealth.bgGradient} backdrop-blur-sm ${clusterHealth.borderColor} shadow-lg`}>
          <CardContent className="p-6">
            <div className="flex items-center gap-3">
              <div className={`w-12 h-12 ${clusterHealth.bgColor} rounded-xl flex items-center justify-center shadow-lg`}>
                <Server className="w-6 h-6 text-white" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Cluster Status</p>
                <div className="flex items-center gap-2">
                  <div className={`w-2 h-2 ${clusterHealth.dotColor} rounded-full ${clusterHealth.status === 'Healthy' ? 'animate-pulse' : ''}`}></div>
                  <span className={`font-bold ${clusterHealth.color}`}>{clusterHealth.status}</span>
                </div>
                {clusterHealth.status !== 'Healthy' && (
                  <p className="text-xs text-muted-foreground mt-1">
                    {clusterHealth.status === 'Critical' 
                      ? `${clusterHealth.failedPods} failed pods, ${clusterHealth.totalNodeCount - clusterHealth.healthyNodeCount} nodes down`
                      : `${clusterHealth.pendingPods} pending pods`}
                  </p>
                )}
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Total Pods */}
        <Card className="bg-gradient-to-r from-blue-500/10 to-blue-600/10 backdrop-blur-sm border-blue-200/20 dark:border-blue-800/20 shadow-lg">
          <CardContent className="p-6">
            <div className="flex items-center gap-3">
              <div className="w-12 h-12 bg-blue-500 rounded-xl flex items-center justify-center shadow-lg">
                <Box className="w-6 h-6 text-white" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Total Pods</p>
                <div className="flex items-center gap-2">
                  <span className="text-2xl font-bold text-blue-600 dark:text-blue-400">{pods.length}</span>
                  <Badge className="bg-blue-100 dark:bg-blue-950 text-blue-800 dark:text-blue-200 text-xs">
                    {clusterHealth.runningPods} Running
                  </Badge>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Nodes */}
        <Card className="bg-gradient-to-r from-purple-500/10 to-purple-600/10 backdrop-blur-sm border-purple-200/20 dark:border-purple-800/20 shadow-lg">
          <CardContent className="p-6">
            <div className="flex items-center gap-3">
              <div className="w-12 h-12 bg-purple-500 rounded-xl flex items-center justify-center shadow-lg">
                <Network className="w-6 h-6 text-white" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Active Nodes</p>
                <div className="flex items-center gap-2">
                  <span className="text-2xl font-bold text-purple-600 dark:text-purple-400">{clusterHealth.nodesData.length}</span>
                  <Badge className="bg-purple-100 dark:bg-purple-950 text-purple-800 dark:text-purple-200 text-xs">
                    All Ready
                  </Badge>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Resource Usage */}
        <Card className="bg-gradient-to-r from-orange-500/10 to-orange-600/10 backdrop-blur-sm border-orange-200/20 dark:border-orange-800/20 shadow-lg">
          <CardContent className="p-6">
            <div className="flex items-center gap-3">
              <div className="w-12 h-12 bg-orange-500 rounded-xl flex items-center justify-center shadow-lg">
                <Activity className="w-6 h-6 text-white" />
              </div>
              <div className="flex-1">
                <p className="text-sm text-muted-foreground mb-2">Avg Resource Usage</p>
                <div className="space-y-2">
                  <div>
                    <div className="flex justify-between text-xs mb-1">
                      <span>CPU</span>
                      <span className="font-mono">{resourceData[0].value}%</span>
                    </div>
                    <Progress value={resourceData[0].value} className="h-1.5" />
                  </div>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Charts Section */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Pod Status Distribution */}
        <Card className="lg:col-span-1 bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Zap className="w-5 h-5 text-chart-1" />
              Pod Status
            </CardTitle>
            <CardDescription>Current distribution of pod states</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="h-48">
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={podStatusData}
                    cx="50%"
                    cy="50%"
                    innerRadius={40}
                    outerRadius={80}
                    paddingAngle={5}
                    dataKey="value"
                  >
                    {podStatusData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.color} />
                    ))}
                  </Pie>
                  <Tooltip 
                    formatter={(value, name) => [`${value} pods`, name]}
                    labelStyle={{ color: '#000' }}
                    contentStyle={{ 
                      backgroundColor: 'white', 
                      border: '1px solid #ccc', 
                      borderRadius: '8px',
                      boxShadow: '0 4px 6px rgba(0, 0, 0, 0.1)'
                    }}
                  />
                </PieChart>
              </ResponsiveContainer>
            </div>
            <div className="grid grid-cols-2 gap-2 mt-4">
              {podStatusData.map((item, index) => (
                <div key={index} className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full" style={{ backgroundColor: item.color }}></div>
                  <span className="text-sm text-muted-foreground">{item.name}: {item.value}</span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        {/* Resource Usage */}
        <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <TrendingUp className="w-5 h-5 text-chart-2" />
              Resource Usage
            </CardTitle>
            <CardDescription>Current cluster resource utilization</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {resourceData.map((resource, index) => (
                <div key={index} className="space-y-2">
                  <div className="flex justify-between items-center">
                    <span className="text-sm font-medium">{resource.name}</span>
                    <span className="text-sm text-muted-foreground">{resource.value}%</span>
                  </div>
                  <Progress 
                    value={resource.value} 
                    className="h-2"
                    style={{ '--progress-background': resource.color } as React.CSSProperties}
                  />
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        {/* Recent Events */}
        <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Clock className="w-5 h-5 text-chart-3" />
              Recent Events
            </CardTitle>
            <CardDescription>Latest cluster activities</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {recentEvents.map((event, index) => (
                <div key={index} className="flex items-start gap-3 p-2 rounded-lg hover:bg-muted/50 transition-colors">
                  <div className={`w-2 h-2 rounded-full mt-2 ${
                    event.status === 'success' ? 'bg-emerald-500' : 
                    event.status === 'error' ? 'bg-red-500' : 'bg-yellow-500'
                  }`}></div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate">
                      <span className="text-muted-foreground">{event.type}</span> {event.resource}
                    </p>
                    <p className="text-xs text-muted-foreground truncate">{event.name}</p>
                    <p className="text-xs text-muted-foreground">{event.time}</p>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="flex items-center gap-4">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search workloads..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="pl-10 h-12 bg-card/80 backdrop-blur-sm border-border/30 shadow-sm hover:shadow-md transition-all"
          />
        </div>
        <div className="flex items-center gap-3">
          <Badge className="bg-slate-50 dark:bg-slate-950 text-slate-700 dark:text-slate-300 border-slate-200 dark:border-slate-800 px-3 py-1">
            <Activity className="w-3 h-3 mr-1" />
            {deployments.length} Deployments
          </Badge>
          <Badge className="bg-slate-50 dark:bg-slate-950 text-slate-700 dark:text-slate-300 border-slate-200 dark:border-slate-800 px-3 py-1">
            <Box className="w-3 h-3 mr-1" />
            {pods.length} Pods
          </Badge>
        </div>
      </div>

      <Tabs defaultValue="pods" className="space-y-6">
        <TabsList className="bg-card/80 backdrop-blur-sm border border-border/30 shadow-lg">
          <TabsTrigger value="deployments" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">
            Deployments
          </TabsTrigger>
          <TabsTrigger value="pods" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">
            Pods
          </TabsTrigger>
          <TabsTrigger value="replicasets" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">
            ReplicaSets
          </TabsTrigger>
          <TabsTrigger value="jobs" className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground">
            Jobs
          </TabsTrigger>
        </TabsList>

        <TabsContent value="deployments">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                  <Box className="w-4 h-4 text-primary-foreground" />
                </div>
                <div>
                  <CardTitle className="text-foreground">Deployments</CardTitle>
                  <CardDescription>Manage your application deployments</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="overflow-hidden rounded-xl border border-border/30">
                <Table>
                  <TableHeader className="bg-muted/80">
                    <TableRow className="border-border/30">
                      <TableHead className="font-semibold text-foreground">Name</TableHead>
                      <TableHead className="font-semibold text-foreground">Ready</TableHead>
                      <TableHead className="font-semibold text-foreground">Up-to-date</TableHead>
                      <TableHead className="font-semibold text-foreground">Available</TableHead>
                      <TableHead className="font-semibold text-foreground">Age</TableHead>
                      <TableHead className="font-semibold text-foreground">Status</TableHead>
                      <TableHead className="font-semibold text-foreground"></TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredDeployments.map((deployment/*, index*/)=> (
                      <TableRow key={deployment.name} className="border-border/30 hover:bg-muted/30 transition-colors">
                        <TableCell className="font-medium text-foreground">{deployment.name}</TableCell>
                        <TableCell>
                          <Badge variant="outline" className="font-mono">
                            {deployment.ready}
                          </Badge>
                        </TableCell>
                        <TableCell className="text-foreground">{deployment.upToDate}</TableCell>
                        <TableCell className="text-foreground">{deployment.available}</TableCell>
                        <TableCell className="text-muted-foreground">{deployment.age}</TableCell>
                        <TableCell>{getStatusBadge(deployment.status)}</TableCell>
                        <TableCell>
                          <div className="flex items-center gap-1">
                            <Button variant="ghost" size="sm" className="w-8 h-8 hover:bg-emerald-100 dark:hover:bg-emerald-950">
                              <Play className="h-3 w-3" />
                            </Button>
                            <Button variant="ghost" size="sm" className="w-8 h-8 hover:bg-slate-100 dark:hover:bg-slate-950">
                              <RotateCcw className="h-3 w-3" />
                            </Button>
                            <Button variant="ghost" size="sm" className="w-8 h-8 hover:bg-muted">
                              <MoreHorizontal className="h-3 w-3" />
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

        <TabsContent value="pods">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl overflow-visible">
            <CardHeader>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                    <Box className="w-4 h-4 text-primary-foreground" />
                  </div>
                  <div>
                    <CardTitle className="text-foreground">Pods</CardTitle>
                    <CardDescription>View and manage individual pod instances</CardDescription>
                  </div>
                </div>
                <Button 
                  className="bg-primary hover:bg-primary/80 text-primary-foreground shadow-lg hover:shadow-xl transition-all gap-2" 
                  onClick={() => setCreatePodDialog(true)}
                >
                  <Plus className="w-4 h-4" />
                  Add Pod
                </Button>
              </div>
            </CardHeader>
            <CardContent className="overflow-visible">
              <div className="rounded-xl border border-border/30 overflow-visible">
                <Table>
                  <TableHeader className="bg-muted/80">
                    <TableRow className="border-border/30">
                      <TableHead className="font-semibold text-foreground">Name</TableHead>
                      <TableHead className="font-semibold text-foreground">Image</TableHead>
                      <TableHead className="font-semibold text-foreground">Labels</TableHead>
                      <TableHead className="font-semibold text-foreground">Node</TableHead>
                      <TableHead className="font-semibold text-foreground">Status</TableHead>
                      <TableHead className="font-semibold text-foreground">CPU Usage</TableHead>
                      <TableHead className="font-semibold text-foreground">Memory Usage</TableHead>
                      <TableHead className="font-semibold text-foreground">Age</TableHead>
                      <TableHead className="font-semibold text-foreground"></TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody className="overflow-visible">
                    {filteredPods.map((pod/*, index*/) => (
                      <TableRow key={pod.name} className="border-border/30 hover:bg-muted/30 transition-colors">
                        <TableCell className="font-medium text-foreground max-w-xs">
                          <Button 
                            variant="ghost" 
                            className="h-auto p-0 font-medium text-chart-1 hover:text-chart-1/80 underline underline-offset-4 cursor-pointer transition-colors"
                            onClick={() => onPodClick?.(pod.name)}
                          >
                            {pod.name}
                          </Button>
                        </TableCell>
                        <TableCell className="font-mono text-sm text-muted-foreground">
                          <Badge variant="outline" className="text-xs">
                            {pod.image}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <div className="flex gap-1 flex-wrap">
                            {Object.entries(pod.labels).map(([key, value]) => (
                              <Badge key={key} variant="secondary" className="text-xs">
                                {key}={value}
                              </Badge>
                            ))}
                          </div>
                        </TableCell>
                        <TableCell>
                          <Badge variant="outline" className="text-xs">
                            {pod.node}
                          </Badge>
                        </TableCell>
                        <TableCell>{getStatusBadge(pod.status)}</TableCell>
                        <TableCell>
                          <div className="flex items-center gap-1">
                            <Cpu className="w-3 h-3 text-muted-foreground" />
                            <span className="font-mono text-sm">{pod.cpuUsage}</span>
                          </div>
                        </TableCell>
                        <TableCell>
                          <div className="flex items-center gap-1">
                            <MemoryStick className="w-3 h-3 text-muted-foreground" />
                            <span className="font-mono text-sm">{pod.memoryUsage}</span>
                          </div>
                        </TableCell>
                        <TableCell className="text-muted-foreground">{pod.age}</TableCell>
                        <TableCell className="relative">
                          <Button 
                            variant="ghost" 
                            size="sm" 
                            className="w-8 h-8 hover:bg-muted"
                            onClick={(e) => toggleMenu(pod.name, e)}
                          >
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

        <TabsContent value="replicasets">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <CardTitle className="text-foreground">ReplicaSets</CardTitle>
              <CardDescription>Manage replica sets for your deployments</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="text-center py-16">
                <div className="w-16 h-16 bg-gradient-to-r from-muted to-muted/80 rounded-2xl flex items-center justify-center mx-auto mb-4">
                  <Box className="w-8 h-8 text-muted-foreground" />
                </div>
                <h3 className="font-semibold text-foreground mb-2">ReplicaSets View</h3>
                <p className="text-muted-foreground">Implementation coming soon</p>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="jobs">
          <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
            <CardHeader>
              <CardTitle className="text-foreground">Jobs</CardTitle>
              <CardDescription>View and manage Kubernetes jobs</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="text-center py-16">
                <div className="w-16 h-16 bg-gradient-to-r from-muted to-muted/80 rounded-2xl flex items-center justify-center mx-auto mb-4">
                  <Activity className="w-8 h-8 text-muted-foreground" />
                </div>
                <h3 className="font-semibold text-foreground mb-2">Jobs View</h3>
                <p className="text-muted-foreground">Implementation coming soon</p>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      {/* Dialog Components */}
      <LogsDialog 
        open={logsDialog.open}
        onOpenChange={(open) => setLogsDialog({ open, podName: logsDialog.podName })}
        podName={logsDialog.podName}
      />

      <TerminalView 
        isVisible={terminalView.open}
        onClose={() => setTerminalView({ open: false, podName: "" })}
        podName={terminalView.podName}
      />

      <YamlEditor 
        open={yamlEditor.open}
        onOpenChange={(open) => setYamlEditor({ open, podName: yamlEditor.podName })}
        podName={yamlEditor.podName}
      />

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={deleteDialog.open} onOpenChange={(open) => setDeleteDialog({ open, podName: deleteDialog.podName })}>
        <AlertDialogContent className="bg-card/95 backdrop-blur-sm">
          <AlertDialogHeader>
            <AlertDialogTitle className="flex items-center gap-2 text-destructive">
              <AlertCircle className="w-5 h-5" />
              Delete Pod
            </AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete pod{" "}
              <span className="font-semibold text-foreground">"{deleteDialog.podName}"</span>?
              <br />
              <span className="text-xs text-muted-foreground mt-2 block">
                This action cannot be undone. The pod will be permanently removed from the cluster.
              </span>
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={() => setDeleteDialog({ open: false, podName: "" })}>
              Cancel
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={handleConfirmDelete}
              className="bg-destructive hover:bg-destructive/80 text-destructive-foreground"
            >
              <Trash2 className="w-4 h-4 mr-2" />
              Delete Pod
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Create Pod Dialog */}
      <CreatePodDialog 
        open={createPodDialog}
        onOpenChange={setCreatePodDialog}
        onCreatePod={(pod) => setPods(prev => [...prev, pod])}
        namespace={namespace}
        setPods={setPods}
      />

      {/* Portal-rendered Menu */}
      {Object.entries(openMenus).map(([podName, isOpen]) => {
        if (!isOpen || !menuPosition) return null;
        
        return createPortal(
          <div 
            key={podName}
            className="fixed w-48 bg-popover border border-border rounded-md shadow-lg z-[9999]"
            style={{
              top: menuPosition.top,
              right: menuPosition.right,
            }}
          >
            <div className="py-1">
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handlePodAction('logs', podName);
                  closeAllMenus();
                }}
                className="flex items-center w-full px-3 py-2 text-sm text-popover-foreground hover:bg-accent cursor-pointer"
              >
                <FileText className="w-4 h-4 mr-2" />
                View Logs
              </button>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handlePodAction('exec', podName);
                  closeAllMenus();
                }}
                className="flex items-center w-full px-3 py-2 text-sm text-popover-foreground hover:bg-accent cursor-pointer"
              >
                <Terminal className="w-4 h-4 mr-2" />
                Exec Shell
              </button>
              <div className="border-t border-border my-1"></div>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handlePodAction('edit', podName);
                  closeAllMenus();
                }}
                className="flex items-center w-full px-3 py-2 text-sm text-popover-foreground hover:bg-accent cursor-pointer"
              >
                <Edit className="w-4 h-4 mr-2" />
                Edit Pod
              </button>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handlePodAction('delete', podName);
                  closeAllMenus();
                }}
                className="flex items-center w-full px-3 py-2 text-sm hover:bg-accent cursor-pointer text-destructive"
              >
                <Trash2 className="w-4 h-4 mr-2" />
                Delete Pod
              </button>
            </div>
          </div>,
          document.body
        );
      })}
    </div>
  );
}