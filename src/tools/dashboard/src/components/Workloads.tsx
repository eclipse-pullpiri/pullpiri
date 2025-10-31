import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "./ui/table";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from "./ui/alert-dialog";
import { Progress } from "./ui/progress";
import { Search, MoreHorizontal, /*Play, Pause, RotateCcw, */Plus, Box, Activity, AlertCircle, Cpu, MemoryStick, FileText, Terminal, Edit, Trash2, Server, Network, TrendingUp, Zap, Clock/*, Users, ChevronDown*/ } from "lucide-react";
import { LogsDialog } from "./LogsDialog";
import { TerminalView } from "./TerminalView";
import { YamlEditor } from "./YamlEditor";
import { CreatePodDialog } from "./CreatePodDialog";
import { PieChart, Pie, Cell, ResponsiveContainer, /*BarChart, Bar, XAxis, YAxis, CartesianGrid, Legend, LineChart, Line, Area, AreaChart,*/ Tooltip } from 'recharts';

// Pod interface
interface Pod {
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

interface WorkloadsProps {
  onPodClick?: (podName: string) => void;
  pods: Pod[];
  setPods: React.Dispatch<React.SetStateAction<Pod[]>>;
  recentEvents: any[];
  setRecentEvents: React.Dispatch<React.SetStateAction<any[]>>;
}

export function Workloads({ onPodClick, pods, setPods, recentEvents, setRecentEvents }: WorkloadsProps) {
  const [searchTerm, setSearchTerm] = useState("");
  const [selectedNode, setSelectedNode] = useState<string>("all");
  const [openMenus, setOpenMenus] = useState<Record<string, boolean>>({});
  const [menuPosition, setMenuPosition] = useState<{
    top: number;
    right: number;
  } | null>(null);


  // LGSI changes:
  const [nodesDataToUse, setNodesDataToUse] = useState<any[]>([]); // Replace mock with fetched data
  const [nodesFetchSuccess, setNodesFetchSuccess] = useState(false);

  useEffect(() => {
    const settingserviceApiUrl = import.meta.env.VITE_SETTING_SERVICE_API_URL;
    const _timeout = import.meta.env.VITE_SETTING_SERVICE_TIMEOUT || 5000;
    if (!settingserviceApiUrl) {
      console.error("VITE_SETTING_SERVICE_API_URL is not defined");
      return;
    }

    // Fetch function
    const fetchNodes = () => {
//      fetch(`${settingserviceApiUrl}/api/v1/metrics/nodes`)
      fetch('/api/v1/metrics/nodes')
        .then(res => res.json())
        .then(data => {
          if (Array.isArray(data) && data.length > 0) {
            setNodesDataToUse(data);
            setNodesFetchSuccess(true);
            console.log("✅ Nodes API fetched:", data);

            setRecentEvents(prev => {
              const newEvent = {
                type: "Fetched",
                resource: "Node",
                name: "Nodes fetch success",
                time: new Date().toLocaleTimeString(),
                status: "success",
              };

              // Find the most recent event for this resource
              const latestSameResourceEvent = prev.find(
                e => e.resource === newEvent.resource
              );

              // Only add if the latest event for this resource is not the same type, status, and name
              if (
                latestSameResourceEvent &&
                latestSameResourceEvent.type === newEvent.type &&
                latestSameResourceEvent.status === newEvent.status &&
                latestSameResourceEvent.name === newEvent.name
              ) {
                return prev;
              }
              return [newEvent, ...prev];
            });
          } else {
            setNodesFetchSuccess(false);
            setNodesDataToUse([]);
          }
        })
        .catch(e => {
          setNodesFetchSuccess(false);
          setNodesDataToUse([]);
          console.error("❌ Nodes API fetch failed:", e);

          setRecentEvents(prev => {
            const newEvent = {
              type: "Fetched",
              resource: "Node",
              name: "Nodes fetch failed",
              time: new Date().toLocaleTimeString(),
              status: "error",
            };

            // Find the most recent event for this resource
            const latestSameResourceEvent = prev.find(
              e => e.resource === newEvent.resource
            );

            // Only add if the latest event for this resource is not the same type, status, and name
            if (
              latestSameResourceEvent &&
              latestSameResourceEvent.type === newEvent.type &&
              latestSameResourceEvent.status === newEvent.status &&
              latestSameResourceEvent.name === newEvent.name
            ) {
              return prev;
            }
            return [newEvent, ...prev];
          });
        });
    };

    // Initial fetch
    fetchNodes();

    // Set up interval
    const intervalId = setInterval(fetchNodes, _timeout);

    // Cleanup on unmount
    return () => clearInterval(intervalId);
  }, []);

  // Dialog states
  const [logsDialog, setLogsDialog] = useState<{
    open: boolean;
    podName: string;
  }>({
    open: false,
    podName: "",
  });
  const [terminalView, setTerminalView] = useState<{
    open: boolean;
    podName: string;
  }>({
    open: false,
    podName: "",
  });
  const [yamlEditor, setYamlEditor] = useState<{
    open: boolean;
    podName: string;
  }>({
    open: false,
    podName: "",
  });

  // Delete confirmation dialog state
  const [deleteDialog, setDeleteDialog] = useState<{
    open: boolean;
    podName: string;
  }>({
    open: false,
    podName: "",
  });

  // Create Pod dialog state
  const [createPodDialog, setCreatePodDialog] = useState(false);

  // CreatePodDialog용 임시 핸들러
  const handleCreatePod = (pod: any) => {
    setPods((prevPods: any[]) => [...prevPods, pod]);
    setCreatePodDialog(false);
  };

  // Toggle menu for specific pod
  const toggleMenu = (podName: string, e: React.MouseEvent) => {
    e.stopPropagation();
    console.log("🔍 Debug: Menu toggle clicked for pod:", podName);

    const rect = e.currentTarget.getBoundingClientRect();
    setMenuPosition({
      top: rect.bottom + window.scrollY,
      right: window.innerWidth - rect.right,
    });

    setOpenMenus((prev) => ({
      ...prev,
      [podName]: !prev[podName],
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
      case "logs":
        console.log(`Opening logs for ${podName}`);
        setLogsDialog({ open: true, podName });
        break;
      case "exec":
        console.log(`Executing shell for ${podName}`);
        setTerminalView({ open: true, podName });
        break;
      case "edit":
        console.log(`Editing ${podName}`);
        setYamlEditor({ open: true, podName });
        break;
      case "delete":
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
    setPods((prevPods) => prevPods.filter((pod) => pod.name !== podName));

    // Close the dialog
    setDeleteDialog({ open: false, podName: "" });

    console.log(`✅ Pod "${podName}" deleted successfully!`);
  };

  // Calculate cluster metrics
  /*const runningPods = pods.filter((pod) => pod.status === "Running").length; //2025-09-23 comment out
  const pendingPods = pods.filter((pod) => pod.status === "Pending").length;
  const failedPods = pods.filter((pod) => pod.status === "Failed").length;
*/
  // Nodes data with more detailed information
  const nodesData = nodesFetchSuccess
    ? nodesDataToUse.map((node: any) => ({
      name: node.node_name,
      pods: pods.filter((pod) => pod.node === node.node_name).length,
      status: "Ready",
      cpu: `${(node.cpu_usage / 100 * node.cpu_count).toFixed(1)}/${node.cpu_count}`,
      memory: `${(node.used_memory / 1024 / 1024 / 1024).toFixed(1)}/${(node.total_memory / 1024 / 1024 / 1024).toFixed(1)}`,
      cpuUsage: Number(node.cpu_usage.toFixed(1)),
      memoryUsage: Number(node.mem_usage.toFixed(1)),
      storageUsage: 0, // If you have storage info, fill here
    }))
    : [];

  // Mock Data
  // {
  //       name: "worker-node-1",
  //       pods: pods.filter((pod) => pod.node === "worker-node-1").length,
  //       status: "Ready",
  //       cpu: "2.4/4",
  //       memory: "3.2/8",
  //       cpuUsage: 60,
  //       memoryUsage: 40,
  //       storageUsage: 35,
  //     },
  //     {
  //       name: "worker-node-2",
  //       pods: pods.filter((pod) => pod.node === "worker-node-2").length,
  //       status: "Ready",
  //       cpu: "1.8/4",
  //       memory: "2.1/8",
  //       cpuUsage: 45,
  //       memoryUsage: 26,
  //       storageUsage: 50,
  //     },
  //     {
  //       name: "worker-node-3",
  //       pods: pods.filter((pod) => pod.node === "worker-node-3").length,
  //       status: "Ready",
  //       cpu: "0.8/4",
  //       memory: "1.5/8",
  //       cpuUsage: 20,
  //       memoryUsage: 19,
  //       storageUsage: 60,
  //     },

  // Get unique node names from pods
  const availableNodes = Array.from(new Set(nodesData.map((node) => node.name)));

  // Get current node data based on selection
  const currentNodeData =
    selectedNode === "all"
      ? null
      : nodesData.find((node) => node.name === selectedNode);

  // Calculate node-specific or cluster-wide metrics
  const currentPods =
    selectedNode === "all"
      ? pods
      : pods.filter((pod) => pod.node === selectedNode);
  const nodeRunningPods = currentPods.filter(
    (pod) => pod.status === "running"
  ).length;
  const nodePendingPods = currentPods.filter(
    (pod) => pod.status === "pending"
  ).length;
  const nodeFailedPods = currentPods.filter(
    (pod) => pod.status === "failed"
  ).length;

  // Calculate cluster health based on actual data
  /* const totalPods = pods.length; //2025-09-23 comment out
  const runningPodPercentage =
    totalPods > 0 ? (runningPods / totalPods) * 100 : 100;
  const healthyNodeCount = nodesData.filter(
    (node) => node.status === "Ready"
  ).length;
  const totalNodeCount = nodesData.length;
  const nodeHealthPercentage =
    totalNodeCount > 0 ? (healthyNodeCount / totalNodeCount) * 100 : 100;
  */
  // Determine cluster health status
  /*const getClusterHealth = () => {   //2025-09-23 comment out
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
  // const clusterHealth = useClusterHealth(pods); // 2025-09-23 comment out

  // Pod status data for chart - node-specific or cluster-wide
  const podStatusData = [
    { name: "Running", value: nodeRunningPods, color: "#10b981" },
    { name: "Pending", value: nodePendingPods, color: "#f59e0b" },
    { name: "Failed", value: nodeFailedPods, color: "#ef4444" },
  ].filter((item) => item.value > 0);

  // Resource usage data - node-specific or cluster average
  const resourceData = currentNodeData
    ? [
      {
        name: "CPU Usage",
        value: currentNodeData.cpuUsage,
        max: 100,
        color: "#3b82f6",
      },
      {
        name: "Memory Usage",
        value: currentNodeData.memoryUsage,
        max: 100,
        color: "#8b5cf6",
      },
      {
        name: "Storage Usage",
        value: currentNodeData.storageUsage,
        max: 100,
        color: "#06b6d4",
      },
    ]
    : [
      {
        name: "CPU Usage",
        value: Math.round(
          nodesData.reduce((acc, node) => acc + node.cpuUsage, 0) /
          nodesData.length
        ),
        max: 100,
        color: "#3b82f6",
      },
      {
        name: "Memory Usage",
        value: Math.round(
          nodesData.reduce((acc, node) => acc + node.memoryUsage, 0) /
          nodesData.length
        ),
        max: 100,
        color: "#8b5cf6",
      },
      {
        name: "Storage Usage",
        value: Math.round(
          nodesData.reduce((acc, node) => acc + node.storageUsage, 0) /
          nodesData.length
        ),
        max: 100,
        color: "#06b6d4",
      },
    ];

  // Recent events
  // const recentEvents = [
  //   {
  //     type: "Created",
  //     resource: "Pod",
  //     name: "frontend-app-7d4b8c9f8d-xyz12",
  //     time: "2m ago",
  //     status: "success",
  //   },
  //   {
  //     type: "Scheduled",
  //     resource: "Pod",
  //     name: "backend-api-5f6a7b8c9d-def56",
  //     time: "5m ago",
  //     status: "success",
  //   },
  //   {
  //     type: "Failed",
  //     resource: "Pod",
  //     name: "database-migration-1a2b3c4d5e-jkl90",
  //     time: "30m ago",
  //     status: "error",
  //   },
  // ];

  // Helper function to get status badge
  const getStatusBadge = (status: string) => {
    const statusConfig = {
      Running: {
        variant: "default" as const,
        className:
          "bg-emerald-100 dark:bg-emerald-950 text-emerald-800 dark:text-emerald-200 border-emerald-200 dark:border-emerald-800",
      },
      Pending: {
        variant: "secondary" as const,
        className:
          "bg-amber-100 dark:bg-amber-950 text-amber-800 dark:text-amber-200 border-amber-200 dark:border-amber-800",
      },
      Failed: {
        variant: "destructive" as const,
        className:
          "bg-red-100 dark:bg-red-950 text-red-800 dark:text-red-200 border-red-200 dark:border-red-800",
      },
      Succeeded: {
        variant: "default" as const,
        className:
          "bg-emerald-100 dark:bg-emerald-950 text-emerald-800 dark:text-emerald-200 border-emerald-200 dark:border-emerald-800",
      },
    };

    const config =
      statusConfig[status as keyof typeof statusConfig] ||
      statusConfig["Pending"];

    return (
      <Badge variant={config.variant} className={config.className}>
        {status}
      </Badge>
    );
  };

  // Filter pods by selected node and search term
  const filteredPods = pods.filter((pod) => {
    const matchesSearch = pod.name
      .toLowerCase()
      .includes(searchTerm.toLowerCase());
    const matchesNode = selectedNode === "all" || pod.node === selectedNode;
    return matchesSearch && matchesNode;
  });

  return (
    <div className="space-y-8" onClick={closeAllMenus}>
      <div className="flex items-center justify-between">
        <div className="relative">
          <div className="flex items-center gap-4 mb-2">
            <div className="w-1 h-8 bg-gradient-to-b from-primary to-primary/80 rounded-full"></div>
            <h1 className="font-bold text-foreground text-[20px]">
              PULLPIRI Workloads
            </h1>
          </div>
          <p className="text-muted-foreground ml-8">
            {selectedNode === "all"
              ? "Manage and monitor pod workloads across all nodes"
              : `Workloads on ${selectedNode}`}
          </p>
        </div>
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2">
              <Server className="w-5 h-5 text-primary" />
              <span className="text-sm font-medium text-foreground">
                Filter by Node:
              </span>
            </div>
            <div className="flex items-center gap-2 p-1 bg-muted/30 rounded-lg border border-border/20">
              <Button
                variant={selectedNode === "all" ? "default" : "ghost"}
                size="sm"
                onClick={() => setSelectedNode("all")}
                className={`h-8 px-3 text-xs font-medium transition-all ${selectedNode === "all"
                  ? "bg-primary text-primary-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground hover:bg-muted/50"
                  }`}
              >
                <Network className="w-3 h-3 mr-1" />
                All Nodes
              </Button>
              {availableNodes.map((nodeName) => (
                <Button
                  key={nodeName}
                  variant={selectedNode === nodeName ? "default" : "ghost"}
                  size="sm"
                  onClick={() => setSelectedNode(nodeName)}
                  className={`h-8 px-3 text-xs font-medium transition-all ${selectedNode === nodeName
                    ? "bg-primary text-primary-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground hover:bg-muted/50"
                    }`}
                >
                  <Server className="w-3 h-3 mr-1" />
                  {nodeName}
                </Button>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Cluster Overview Dashboard */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        {/* Total Pods */}
        <Card className="bg-gradient-to-r from-blue-500/10 to-blue-600/10 backdrop-blur-sm border-blue-200/20 dark:border-blue-800/20 shadow-lg">
          <CardContent className="p-6">
            <div className="flex items-center gap-3">
              <div className="w-12 h-12 bg-blue-500 rounded-xl flex items-center justify-center shadow-lg">
                <Box className="w-6 h-6 text-white" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">
                  {selectedNode === "all"
                    ? "Total Pods"
                    : `Pods on ${selectedNode}`}
                </p>
                <div className="flex items-center gap-2">
                  <span className="text-2xl font-bold text-blue-600 dark:text-blue-400">
                    {currentPods.length}
                  </span>
                  <Badge className="bg-blue-100 dark:bg-blue-950 text-blue-800 dark:text-blue-200 text-xs">
                    {nodeRunningPods} Running
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
                <p className="text-sm text-muted-foreground">
                  {selectedNode === "all" ? "Active Nodes" : "Node Status"}
                </p>
                <div className="flex items-center gap-2">
                  {selectedNode === "all" ? (
                    <>
                      <span className="text-2xl font-bold text-purple-600 dark:text-purple-400">
                        {nodesData.length}
                      </span>
                      <Badge className="bg-purple-100 dark:bg-purple-950 text-purple-800 dark:text-purple-200 text-xs">
                        All Ready
                      </Badge>
                    </>
                  ) : (
                    <>
                      <span className="text-2xl font-bold text-purple-600 dark:text-purple-400">
                        {currentNodeData?.status}
                      </span>
                      <Badge className="bg-purple-100 dark:bg-purple-950 text-purple-800 dark:text-purple-200 text-xs">
                        {currentNodeData?.pods} Pods
                      </Badge>
                    </>
                  )}
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
                <p className="text-sm text-muted-foreground mb-2">
                  {selectedNode === "all"
                    ? "Avg Resource Usage"
                    : `${selectedNode} Resources`}
                </p>
                <div className="space-y-2">
                  <div>
                    <div className="flex justify-between text-xs mb-1">
                      <span>CPU</span>
                      <span className="font-mono">
                        {resourceData[0].value}%
                      </span>
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
            <CardDescription>
              {selectedNode === "all"
                ? "Current distribution of pod states across cluster"
                : `Pod states on ${selectedNode}`}
            </CardDescription>
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
                    formatter={(value: any, name: any) => [`${value} pods`, name]}
                    labelStyle={{ color: "#000" }}
                    contentStyle={{
                      backgroundColor: "white",
                      border: "1px solid #ccc",
                      borderRadius: "8px",
                      boxShadow: "0 4px 6px rgba(0, 0, 0, 0.1)",
                    }}
                  />
                </PieChart>
              </ResponsiveContainer>
            </div>
            <div className="grid grid-cols-2 gap-2 mt-4">
              {podStatusData.map((item, index) => (
                <div key={index} className="flex items-center gap-2">
                  <div
                    className="w-3 h-3 rounded-full"
                    style={{ backgroundColor: item.color }}
                  ></div>
                  <span className="text-sm text-muted-foreground">
                    {item.name}: {item.value}
                  </span>
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
            <CardDescription>
              {selectedNode === "all"
                ? "Average cluster resource utilization"
                : `Resource utilization on ${selectedNode}`}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {resourceData.map((resource, index) => (
                <div key={index} className="space-y-2">
                  <div className="flex justify-between items-center">
                    <span className="text-sm font-medium">{resource.name}</span>
                    <span className="text-sm text-muted-foreground">
                      {resource.value}%
                    </span>
                  </div>
                  <Progress
                    value={resource.value}
                    className="h-2"
                    style={
                      {
                        "--progress-background": resource.color,
                      } as React.CSSProperties
                    }
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
              {recentEvents.slice(0, 3).map((event, index) => (
                <div
                  key={index}
                  className="flex items-start gap-3 p-2 rounded-lg hover:bg-muted/50 transition-colors"
                >
                  <div
                    className={`w-2 h-2 rounded-full mt-2 ${event.status === "success"
                      ? "bg-emerald-500"
                      : event.status === "error"
                        ? "bg-red-500"
                        : "bg-yellow-500"
                      }`}
                  ></div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate">
                      <span className="text-muted-foreground">
                        {event.type}
                      </span>{" "}
                      {event.resource}
                    </p>
                    <p className="text-xs text-muted-foreground truncate">
                      {event.name}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      {event.time}
                    </p>
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
            <Box className="w-3 h-3 mr-1" />
            {filteredPods.length} Pods
            {selectedNode !== "all" && (
              <span className="ml-1 text-xs opacity-70">on {selectedNode}</span>
            )}
          </Badge>
        </div>
      </div>

      {/* Pods Table */}
      <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl overflow-visible">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                <Box className="w-4 h-4 text-primary-foreground" />
              </div>
              <div>
                <CardTitle className="text-foreground">Pods</CardTitle>
                <CardDescription>
                  View and manage individual pod instances
                </CardDescription>
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
                  <TableHead className="font-semibold text-foreground">
                    Name
                  </TableHead>
                  <TableHead className="font-semibold text-foreground">
                    Image
                  </TableHead>
                  <TableHead className="font-semibold text-foreground">
                    Labels
                  </TableHead>
                  <TableHead className="font-semibold text-foreground">
                    Node
                  </TableHead>
                  <TableHead className="font-semibold text-foreground">
                    Status
                  </TableHead>
                  <TableHead className="font-semibold text-foreground">
                    CPU Usage
                  </TableHead>
                  <TableHead className="font-semibold text-foreground">
                    Memory Usage
                  </TableHead>
                  <TableHead className="font-semibold text-foreground">
                    Age
                  </TableHead>
                  <TableHead className="font-semibold text-foreground"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody className="overflow-visible">
                {filteredPods.map(
                  (pod, idx) => (
                    <TableRow
                       key={`${pod.name}-${pod.ip || idx}`}
                      className="border-border/30 hover:bg-muted/30 transition-colors"
                    >
                      <TableCell className="font-medium text-foreground max-w-xs">
                        <Button
                          variant="ghost"
                          className="h-auto p-0 font-medium text-[rgba(81,127,255,1)] dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 underline underline-offset-4 cursor-pointer transition-colors"
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
                            <Badge
                              key={key}
                              variant="secondary"
                              className="text-xs"
                            >
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
                          <span className="font-mono text-sm">
                            {pod.cpuUsage}
                          </span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-1">
                          <MemoryStick className="w-3 h-3 text-muted-foreground" />
                          <span className="font-mono text-sm">
                            {pod.memoryUsage}
                          </span>
                        </div>
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {pod.age}
                      </TableCell>
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
                  )
                )}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>

      {/* Dialog Components */}
      <LogsDialog
        open={logsDialog.open}
        onOpenChange={(open: any) =>
          setLogsDialog({ open, podName: logsDialog.podName })
        }
        podName={logsDialog.podName}
      />

      <TerminalView
        isVisible={terminalView.open}
        onClose={() => setTerminalView({ open: false, podName: "" })}
        podName={terminalView.podName}
      />

      <YamlEditor
        open={yamlEditor.open}
        onOpenChange={(open: any) =>
          setYamlEditor({ open, podName: yamlEditor.podName })
        }
        podName={yamlEditor.podName}
      />

      {/* Delete Confirmation Dialog */}
      <AlertDialog
        open={deleteDialog.open}
        onOpenChange={(open: any) =>
          setDeleteDialog({ open, podName: deleteDialog.podName })
        }
      >
        <AlertDialogContent className="bg-card/95 backdrop-blur-sm">
          <AlertDialogHeader>
            <AlertDialogTitle className="flex items-center gap-2 text-destructive">
              <AlertCircle className="w-5 h-5" />
              Delete Pod
            </AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete pod{" "}
              <span className="font-semibold text-foreground">
                "{deleteDialog.podName}"
              </span>
              ?
              <br />
              <span className="text-xs text-muted-foreground mt-2 block">
                This action cannot be undone. The pod will be permanently
                removed from the cluster.
              </span>
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel
              onClick={() => setDeleteDialog({ open: false, podName: "" })}
            >
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
        onCreatePod={handleCreatePod}
        onSuccess={(newPod: any) => {
          setPods((prevPods) => [...prevPods, newPod]);
          setCreatePodDialog(false);
        }}
      />

      {/* Context Menu Portal */}
      {openMenus &&
        Object.keys(openMenus).length > 0 &&
        menuPosition &&
        createPortal(
          <>
            {Object.entries(openMenus).map(
              ([podName, isOpen]) =>
                isOpen && (
                  <div
                    key={podName}
                    className="fixed z-50 bg-card/95 backdrop-blur-sm border border-border/50 rounded-lg shadow-xl py-2 min-w-[160px]"
                    style={{
                      top: menuPosition.top,
                      right: menuPosition.right,
                    }}
                    onClick={(e) => e.stopPropagation()}
                  >
                    <button
                      className="w-full px-3 py-2 text-left text-sm hover:bg-muted/50 transition-colors flex items-center gap-2"
                      onClick={() => {
                        handlePodAction("logs", podName);
                        closeAllMenus();
                      }}
                    >
                      <FileText className="w-4 h-4" />
                      View Logs
                    </button>
                    <button
                      className="w-full px-3 py-2 text-left text-sm hover:bg-muted/50 transition-colors flex items-center gap-2"
                      onClick={() => {
                        handlePodAction("exec", podName);
                        closeAllMenus();
                      }}
                    >
                      <Terminal className="w-4 h-4" />
                      Exec Shell
                    </button>
                    <button
                      className="w-full px-3 py-2 text-left text-sm hover:bg-muted/50 transition-colors flex items-center gap-2"
                      onClick={() => {
                        handlePodAction("edit", podName);
                        closeAllMenus();
                      }}
                    >
                      <Edit className="w-4 h-4" />
                      Edit YAML
                    </button>
                    <div className="border-t border-border/50 my-1"></div>
                    <button
                      className="w-full px-3 py-2 text-left text-sm hover:bg-destructive/10 text-destructive transition-colors flex items-center gap-2"
                      onClick={() => {
                        handlePodAction("delete", podName);
                        closeAllMenus();
                      }}
                    >
                      <Trash2 className="w-4 h-4" />
                      Delete Pod
                    </button>
                  </div>
                )
            )}
          </>,
          document.body
        )}
    </div>
  );
}
