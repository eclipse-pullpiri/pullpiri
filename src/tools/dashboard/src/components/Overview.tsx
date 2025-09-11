import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Badge } from "./ui/badge";
//import { Progress } from "./ui/progress";
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer,/* BarChart, Bar,*/ PieChart, Pie, Cell/*, Legend*/ } from "recharts";
import { Server, Box, Network, Database, /*AlertTriangle, CheckCircle, */TrendingUp, Activity, Zap, HardDrive, Cpu, MemoryStick } from "lucide-react";

interface OverviewProps {
  namespace: string;
}

export function Overview({ namespace }: OverviewProps) {
  // Mock data
  const clusterStats = {
    nodes: { total: 3, ready: 3, notReady: 0 },
    pods: { total: 42, running: 38, pending: 2, failed: 2 },
    services: { total: 15, active: 15 },
    deployments: { total: 12, ready: 10, updating: 2 }
  };

  const cpuUsageData = [
    { time: "00:00", usage: 45, memory: 55 },
    { time: "04:00", usage: 52, memory: 48 },
    { time: "08:00", usage: 68, memory: 72 },
    { time: "12:00", usage: 75, memory: 81 },
    { time: "16:00", usage: 62, memory: 69 },
    { time: "20:00", usage: 58, memory: 63 },
  ];

  const nodeData = [
    { name: "node-1", cpu: 45, memory: 67, pods: 14, status: "healthy" },
    { name: "node-2", cpu: 62, memory: 54, pods: 16, status: "healthy" },
    { name: "node-3", cpu: 38, memory: 71, pods: 12, status: "healthy" },
  ];

  const podDistribution = [
    { name: "Running", value: 38, color: "#10b981" },
    { name: "Pending", value: 2, color: "#f59e0b" },
    { name: "Failed", value: 2, color: "#ef4444" },
  ];

  const StatCard = ({ title, value, subtitle, icon: Icon, gradient, trend }: any) => (
    <Card className="relative overflow-hidden bg-card/80 backdrop-blur-sm border-border/20 shadow-xl hover:shadow-2xl transition-all duration-300 group">
      <div className={`absolute inset-0 bg-gradient-to-r ${gradient} opacity-5 group-hover:opacity-10 transition-opacity dark:opacity-10 dark:group-hover:opacity-20`}></div>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-3 relative z-10">
        <div>
          <CardTitle className="text-sm font-semibold text-muted-foreground">{title}</CardTitle>
          <div className="font-bold text-foreground mt-1">{value}</div>
        </div>
        <div className={`w-14 h-14 rounded-2xl bg-gradient-to-r ${gradient} flex items-center justify-center shadow-lg`}>
          <Icon className="h-7 w-7 text-white" />
        </div>
      </CardHeader>
      <CardContent className="relative z-10">
        <div className="flex items-center gap-2 text-sm">
          {trend && (
            <div className="flex items-center gap-1">
              <TrendingUp className="w-3 h-3 text-emerald-500" />
              <span className="text-emerald-600 dark:text-emerald-400 font-medium">+12%</span>
            </div>
          )}
          <span className="text-muted-foreground">{subtitle}</span>
        </div>
      </CardContent>
    </Card>
  );

  return (
    <div className="space-y-6 lg:space-y-8">
      <div className="relative">
        <div className="flex items-center gap-3 lg:gap-4 mb-2">
          <div className="w-1 h-6 lg:h-8 bg-gradient-to-b from-primary to-primary/80 rounded-full"></div>
          <h1 className="text-xl lg:text-2xl xl:text-3xl font-bold text-foreground">
            PULLPIRI Cluster Overview
          </h1>
        </div>
        <p className="text-sm lg:text-base text-muted-foreground ml-6 lg:ml-8">
          Namespace: <span className="font-semibold text-primary">"{namespace}"</span> • 
          Last updated: <span className="font-medium">2 minutes ago</span>
        </p>
      </div>

      {/* Enhanced Stats Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4 lg:gap-6">
        <StatCard
          title="Cluster Nodes"
          value={clusterStats.nodes.total}
          subtitle={`${clusterStats.nodes.ready} nodes ready`}
          icon={Server}
          gradient="from-slate-600 to-slate-700"
          trend={true}
        />
        <StatCard
          title="Active Pods"
          value={clusterStats.pods.total}
          subtitle={`${clusterStats.pods.running} running, ${clusterStats.pods.pending} pending`}
          icon={Box}
          gradient="from-slate-600 to-slate-700"
          trend={true}
        />
        <StatCard
          title="Services"
          value={clusterStats.services.total}
          subtitle="All services active"
          icon={Network}
          gradient="from-slate-600 to-slate-700"
          trend={false}
        />
        <StatCard
          title="Deployments"
          value={clusterStats.deployments.total}
          subtitle={`${clusterStats.deployments.ready} ready, ${clusterStats.deployments.updating} updating`}
          icon={Database}
          gradient="from-slate-600 to-slate-700"
          trend={true}
        />
      </div>

      {/* Enhanced Charts Section */}
      <div className="grid grid-cols-1 xl:grid-cols-3 gap-4 lg:gap-6">
        <Card className="xl:col-span-2 bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
          <CardHeader className="pb-4 lg:pb-6">
            <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4">
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                  <Activity className="w-4 h-4 text-primary-foreground" />
                </div>
                <div>
                  <CardTitle className="text-base lg:text-lg text-foreground">Cluster Resource Usage Trends</CardTitle>
                  <CardDescription className="text-sm">Average CPU and Memory usage across all nodes over the last 24 hours</CardDescription>
                </div>
              </div>
              {/* Chart Legend */}
              <div className="flex flex-wrap items-center gap-4 lg:gap-6">
                <div className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full bg-slate-600"></div>
                  <div className="flex items-center gap-1">
                    <Cpu className="w-4 h-4 text-slate-600 dark:text-slate-400" />
                    <span className="text-xs lg:text-sm font-medium text-foreground">Cluster CPU</span>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full bg-slate-500"></div>
                  <div className="flex items-center gap-1">
                    <MemoryStick className="w-4 h-4 text-slate-600 dark:text-slate-400" />
                    <span className="text-xs lg:text-sm font-medium text-foreground">Cluster Memory</span>
                  </div>
                </div>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="h-64 lg:h-80">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={cpuUsageData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
                  <XAxis dataKey="time" stroke="hsl(var(--muted-foreground))" fontSize={12} />
                  <YAxis stroke="hsl(var(--muted-foreground))" label={{ value: 'Usage (%)', angle: -90, position: 'insideLeft' }} fontSize={12} />
                  <Tooltip 
                    contentStyle={{ 
                      backgroundColor: 'hsl(var(--card))', 
                      border: '1px solid hsl(var(--border))', 
                      borderRadius: '12px',
                      boxShadow: '0 20px 25px -5px rgba(0, 0, 0, 0.1)',
                      color: 'hsl(var(--foreground))',
                      fontSize: '12px'
                    }}
                    labelFormatter={(label) => `Time: ${label}`}
                    formatter={(value, name) => [
                      `${value}%`, 
                      name === 'usage' ? 'Cluster CPU Usage' : 'Cluster Memory Usage'
                    ]}
                  />
                  <Line 
                    type="monotone" 
                    dataKey="usage" 
                    stroke="#475569" 
                    strokeWidth={2}
                    dot={{ fill: '#475569', strokeWidth: 1, r: 3 }}
                    name="Cluster CPU Usage"
                    activeDot={{ r: 4, stroke: '#475569', strokeWidth: 2, fill: '#ffffff' }}
                  />
                  <Line 
                    type="monotone" 
                    dataKey="memory" 
                    stroke="#64748b" 
                    strokeWidth={2}
                    dot={{ fill: '#64748b', strokeWidth: 1, r: 3 }}
                    name="Cluster Memory Usage"
                    activeDot={{ r: 4, stroke: '#64748b', strokeWidth: 2, fill: '#ffffff' }}
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
            
            {/* Additional Info Cards Below Chart */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3 lg:gap-4 mt-4 lg:mt-6">
              <div className="bg-slate-50 dark:bg-slate-950/30 p-3 lg:p-4 rounded-xl border border-slate-200 dark:border-slate-800">
                <div className="flex items-center gap-2 lg:gap-3">
                  <div className="w-6 lg:w-8 h-6 lg:h-8 bg-slate-600 rounded-lg flex items-center justify-center">
                    <Cpu className="w-3 lg:w-4 h-3 lg:h-4 text-white" />
                  </div>
                  <div>
                    <p className="text-xs lg:text-sm font-medium text-slate-700 dark:text-slate-300">Cluster CPU Usage</p>
                    <p className="text-sm lg:text-base font-bold text-slate-800 dark:text-slate-200">58%</p>
                    <p className="text-xs text-slate-600 dark:text-slate-400">Average across all nodes</p>
                  </div>
                </div>
              </div>
              <div className="bg-slate-50 dark:bg-slate-950/30 p-3 lg:p-4 rounded-xl border border-slate-200 dark:border-slate-800">
                <div className="flex items-center gap-2 lg:gap-3">
                  <div className="w-6 lg:w-8 h-6 lg:h-8 bg-slate-600 rounded-lg flex items-center justify-center">
                    <MemoryStick className="w-3 lg:w-4 h-3 lg:h-4 text-white" />
                  </div>
                  <div>
                    <p className="text-xs lg:text-sm font-medium text-slate-700 dark:text-slate-300">Cluster Memory Usage</p>
                    <p className="text-sm lg:text-base font-bold text-slate-800 dark:text-slate-200">63%</p>
                    <p className="text-xs text-slate-600 dark:text-slate-400">Average across all nodes</p>
                  </div>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
          <CardHeader>
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                <Box className="w-4 h-4 text-primary-foreground" />
              </div>
              <div>
                <CardTitle className="text-base lg:text-lg text-foreground">Pod Distribution</CardTitle>
                <CardDescription className="text-sm">Current pod status breakdown</CardDescription>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="h-48 lg:h-64">
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={podDistribution}
                    cx="50%"
                    cy="50%"
                    innerRadius={40}
                    outerRadius={70}
                    paddingAngle={5}
                    dataKey="value"
                  >
                    {podDistribution.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.color} />
                    ))}
                  </Pie>
                  <Tooltip 
                    contentStyle={{
                      backgroundColor: 'hsl(var(--card))',
                      border: '1px solid hsl(var(--border))',
                      borderRadius: '8px',
                      color: 'hsl(var(--foreground))',
                      fontSize: '12px'
                    }}
                  />
                </PieChart>
              </ResponsiveContainer>
            </div>
            <div className="space-y-2 mt-4">
              {podDistribution.map((item, index) => (
                <div key={index} className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <div className="w-3 h-3 rounded-full" style={{ backgroundColor: item.color }}></div>
                    <span className="text-xs lg:text-sm font-medium text-foreground">{item.name}</span>
                  </div>
                  <span className="text-xs lg:text-sm font-bold text-foreground">{item.value}</span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Enhanced Node Status */}
      <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
        <CardHeader>
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
              <Server className="w-4 h-4 text-primary-foreground" />
            </div>
            <div>
              <CardTitle className="text-base lg:text-lg text-foreground">Node Performance</CardTitle>
              <CardDescription className="text-sm">Real-time resource utilization across cluster nodes</CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="space-y-4 lg:space-y-6">
            {nodeData.map((node/*, index*/) => (
              <div key={node.name} className="p-4 lg:p-6 bg-gradient-to-r from-accent/50 to-muted/50 rounded-2xl border border-border/30">
                <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-4">
                  <div className="flex items-center gap-3">
                    <div className="w-10 lg:w-12 h-10 lg:h-12 bg-primary rounded-xl flex items-center justify-center">
                      <HardDrive className="w-5 lg:w-6 h-5 lg:h-6 text-primary-foreground" />
                    </div>
                    <div>
                      <h3 className="text-sm lg:text-base font-bold text-foreground">{node.name}</h3>
                      <div className="flex items-center gap-2">
                        <div className="w-2 h-2 bg-emerald-500 rounded-full"></div>
                        <span className="text-xs lg:text-sm text-muted-foreground">Healthy • {node.pods} pods</span>
                      </div>
                    </div>
                  </div>
                  <Badge className="bg-emerald-100 dark:bg-emerald-950 text-emerald-800 dark:text-emerald-200 border-emerald-200 dark:border-emerald-800 self-start sm:self-auto">
                    <Zap className="w-3 h-3 mr-1" />
                    Active
                  </Badge>
                </div>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 lg:gap-6">
                  <div>
                    <div className="flex justify-between items-center text-xs lg:text-sm mb-2">
                      <div className="flex items-center gap-2">
                        <Cpu className="w-3 lg:w-4 h-3 lg:h-4 text-slate-600 dark:text-slate-400" />
                        <span className="font-medium text-foreground">CPU Usage</span>
                      </div>
                      <span className="font-bold text-foreground">{node.cpu}%</span>
                    </div>
                    <div className="h-2 lg:h-3 bg-muted rounded-full overflow-hidden">
                      <div 
                        className="h-full bg-slate-600 rounded-full transition-all duration-300"
                        style={{ width: `${node.cpu}%` }}
                      />
                    </div>
                  </div>
                  <div>
                    <div className="flex justify-between items-center text-xs lg:text-sm mb-2">
                      <div className="flex items-center gap-2">
                        <MemoryStick className="w-3 lg:w-4 h-3 lg:h-4 text-slate-600 dark:text-slate-400" />
                        <span className="font-medium text-foreground">Memory Usage</span>
                      </div>
                      <span className="font-bold text-foreground">{node.memory}%</span>
                    </div>
                    <div className="h-2 lg:h-3 bg-muted rounded-full overflow-hidden">
                      <div 
                        className="h-full bg-slate-500 rounded-full transition-all duration-300"
                        style={{ width: `${node.memory}%` }}
                      />
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}