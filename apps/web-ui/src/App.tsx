import { useEffect } from "react";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Form, FormControl, FormField, FormItem, FormLabel } from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useComplianceSnapshot, useInventory, usePlanJob, useSchedules } from "@/hooks/useData";
import { useToast } from "@/hooks/use-toast";

const jobSchema = z.object({
  name: z.string().min(1, "Job name is required"),
  type: z.enum(["command", "config", "compliance"]),
  target: z.string(),
  content: z.string().min(1, "Commands or config snippet required"),
  dryRun: z.boolean(),
});

const scheduleSchema = z.object({
  name: z.string().min(1, "Schedule name is required"),
  cron: z.string().min(1, "Cron expression is required"),
});

type JobForm = z.infer<typeof jobSchema>;
type ScheduleForm = z.infer<typeof scheduleSchema>;

function App() {
  const { toast } = useToast();
  const inventory = useInventory();
  const schedules = useSchedules();
  const planJob = usePlanJob();
  const compliance = useComplianceSnapshot();

  const jobForm = useForm<JobForm>({
    resolver: zodResolver(jobSchema),
    defaultValues: {
      name: "",
      type: "command",
      target: "",
      content: "",
      dryRun: false,
    },
  });

  const scheduleForm = useForm<ScheduleForm>({
    resolver: zodResolver(scheduleSchema),
    defaultValues: {
      name: "",
      cron: "",
    },
  });

  useEffect(() => {
    if (planJob.data?.message) {
      toast({
        title: planJob.data.status === "planned" ? "Job planned" : "Job failed",
        description: planJob.data.message,
        variant: planJob.data.status === "planned" ? "default" : "destructive",
      });
    }
  }, [planJob.data, toast]);

  const handlePlanJob = jobForm.handleSubmit((values) => {
    planJob.mutate(values);
  });

  const handleSchedule = scheduleForm.handleSubmit((values) => {
    toast({
      title: "Schedule saved",
      description: `${values.name || "Unnamed"} @ ${values.cron}`,
    });
    scheduleForm.reset();
  });

  return (
    <TooltipProvider delayDuration={100}>
      <div className="min-h-screen bg-background text-foreground">
        <header className="border-b border-border/60 bg-background/70 px-8 py-6 backdrop-blur">
          <div className="mx-auto flex max-w-6xl items-center justify-between">
            <h1 className="text-3xl font-semibold tracking-tight text-foreground drop-shadow">
              Network Automation Control Center
            </h1>
            <Badge variant="secondary">Dark theme</Badge>
          </div>
        </header>

        <main className="mx-auto max-w-6xl space-y-6 px-6 py-10">
          <div className="grid gap-6 lg:grid-cols-4">
            <Card className="lg:col-span-1 bg-card/80 shadow-card backdrop-blur-sm">
              <CardHeader className="pb-3">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm font-semibold uppercase tracking-wide text-muted-foreground/80">
                      Inventory
                    </p>
                    <CardTitle>Devices</CardTitle>
                  </div>
                  <Badge variant="secondary">
                    {inventory.isLoading ? "Loading" : "Live"}
                  </Badge>
                </div>
              </CardHeader>
              <CardContent>
                <div className="overflow-hidden rounded-lg border border-border/70 bg-background/30">
                  <div className="grid grid-cols-3 px-3 py-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    <span>Device</span>
                    <span>Type</span>
                    <span>Tags</span>
                  </div>
                  <div className="border-t border-border/60 px-3 py-8 text-sm text-muted-foreground">
                    {inventory.isLoading && "Loading inventory..."}
                    {inventory.error && "Failed to load inventory."}
                    {!inventory.isLoading && !inventory.error && (
                      <div className="space-y-2">
                        {inventory.data?.length ? (
                          inventory.data.map((item) => (
                            <div
                              key={item.id}
                              className="grid grid-cols-3 items-center rounded-md bg-background/40 px-2 py-2 text-foreground"
                            >
                              <span className="font-medium">{item.name}</span>
                              <span className="text-muted-foreground">
                                {item.type}
                              </span>
                              <span className="text-muted-foreground">
                                {item.tags.join(", ")}
                              </span>
                            </div>
                          ))
                        ) : (
                          "No devices yet. Connect your inventory API."
                        )}
                      </div>
                    )}
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card className="lg:col-span-2 bg-card/80 shadow-card backdrop-blur-sm">
              <CardHeader className="pb-4">
                <CardTitle>Job Wizard</CardTitle>
                <CardDescription className="text-muted-foreground">
                  Plan and submit automation jobs for selected targets.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                <Form {...jobForm}>
                  <form className="space-y-3" onSubmit={handlePlanJob}>
                    <FormField
                      control={jobForm.control}
                      name="name"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>Job name</FormLabel>
                          <FormControl>
                            <Input placeholder="Command batch" {...field} />
                          </FormControl>
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={jobForm.control}
                      name="type"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>Job type</FormLabel>
                          <Select
                            onValueChange={field.onChange}
                            defaultValue={field.value}
                          >
                            <FormControl>
                              <SelectTrigger>
                                <SelectValue placeholder="Select a type" />
                              </SelectTrigger>
                            </FormControl>
                            <SelectContent>
                              <SelectItem value="command">Command batch</SelectItem>
                              <SelectItem value="config">Config snippet</SelectItem>
                              <SelectItem value="compliance">
                                Compliance scan
                              </SelectItem>
                            </SelectContent>
                          </Select>
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={jobForm.control}
                      name="target"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>Target filter (e.g. site:oslo)</FormLabel>
                          <FormControl>
                            <Input placeholder="site:oslo" {...field} />
                          </FormControl>
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={jobForm.control}
                      name="content"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>Commands or config snippet</FormLabel>
                          <FormControl>
                            <Textarea
                              className="min-h-[120px]"
                              placeholder="enter commands..."
                              {...field}
                            />
                          </FormControl>
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={jobForm.control}
                      name="dryRun"
                      render={({ field }) => (
                        <FormItem className="flex flex-row items-center space-x-3 space-y-0 rounded-md border border-border/60 bg-muted/20 px-3 py-2">
                          <FormControl>
                            <Checkbox
                              checked={field.value}
                              onCheckedChange={(checked) =>
                                field.onChange(checked === true)
                              }
                            />
                          </FormControl>
                          <FormLabel className="text-sm font-medium text-muted-foreground">
                            Dry run
                          </FormLabel>
                        </FormItem>
                      )}
                    />
                    <div className="flex gap-3">
                      <Button className="flex-1" type="submit" disabled={planJob.isPending}>
                        {planJob.isPending ? "Planning..." : "Plan job"}
                      </Button>
                      <Button
                        variant="secondary"
                        className="flex-1"
                        type="button"
                        onClick={() => jobForm.reset()}
                      >
                        Reset
                      </Button>
                    </div>
                    <div className="rounded-lg border border-border/70 bg-muted/30 px-3 py-2 font-mono text-sm text-muted-foreground">
                      {planJob.isPending && "Planning job..."}
                      {!planJob.isPending &&
                        (planJob.data?.message || "Awaiting job...")}
                    </div>
                  </form>
                </Form>
              </CardContent>
            </Card>

            <Card className="lg:col-span-1 bg-card/80 shadow-card backdrop-blur-sm">
              <CardHeader className="pb-4">
                <CardTitle>Scheduling</CardTitle>
                <CardDescription className="text-muted-foreground">
                  Add cron schedules for recurring jobs.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                <Form {...scheduleForm}>
                  <form className="space-y-3" onSubmit={handleSchedule}>
                    <FormField
                      control={scheduleForm.control}
                      name="name"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>Schedule name</FormLabel>
                          <FormControl>
                            <Input placeholder="Nightly backup" {...field} />
                          </FormControl>
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={scheduleForm.control}
                      name="cron"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>Cron expression (e.g. 0 2 * * *)</FormLabel>
                          <FormControl>
                            <Input placeholder="0 2 * * *" {...field} />
                          </FormControl>
                        </FormItem>
                      )}
                    />
                    <Button className="w-full" type="submit">
                      Add schedule
                    </Button>
                  </form>
                </Form>
                <div className="rounded-lg border border-border/70 bg-muted/30 px-3 py-2 font-mono text-sm text-muted-foreground">
                  {schedules.isLoading && "Loading schedules..."}
                  {schedules.error && "Failed to load schedules."}
                  {!schedules.isLoading && !schedules.error && (
                    <div className="space-y-2">
                      {schedules.data?.length ? (
                        schedules.data.map((schedule) => (
                          <div
                            key={schedule.id}
                            className="flex items-center justify-between rounded-md bg-background/40 px-3 py-2"
                          >
                            <div>
                              <p className="font-semibold">{schedule.name}</p>
                              <p className="text-xs text-muted-foreground">
                                {schedule.cron}
                              </p>
                            </div>
                            <Badge variant="secondary">Active</Badge>
                          </div>
                        ))
                      ) : (
                        "No schedules yet."
                      )}
                    </div>
                  )}
                </div>
              </CardContent>
            </Card>
          </div>

          <Card className="bg-card/80 shadow-card backdrop-blur-sm">
            <CardHeader className="flex flex-row items-center justify-between pb-3">
              <CardTitle>Compliance</CardTitle>
              <Button
                size="sm"
                onClick={() => compliance.mutate()}
                disabled={compliance.isPending}
              >
                {compliance.isPending ? "Refreshing..." : "Refresh snapshot"}
              </Button>
            </CardHeader>
            <CardContent>
              <div className="rounded-lg border border-border/70 bg-muted/30 px-3 py-2 font-mono text-sm text-muted-foreground">
                {compliance.isPending
                  ? "Requesting snapshot..."
                  : compliance.data?.message || "Loading..."}
              </div>
            </CardContent>
          </Card>
        </main>
      </div>
    </TooltipProvider>
  );
}

export default App;
