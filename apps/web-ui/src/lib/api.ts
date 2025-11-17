// Temporary mock API layer. Replace implementations with real HTTP calls.

export type InventoryItem = {
  id: string;
  name: string;
  type: string;
  tags: string[];
};

export type JobPlanRequest = {
  name: string;
  type: "command" | "config" | "compliance";
  target: string;
  content: string;
  dryRun: boolean;
};

export type JobPlanResponse = {
  status: "planned" | "error";
  message: string;
};

export type Schedule = {
  id: string;
  name: string;
  cron: string;
};

export type ComplianceSnapshot = {
  status: "idle" | "running" | "complete";
  message: string;
};

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const mockInventory: InventoryItem[] = [
  { id: "r1", name: "router-1", type: "Router", tags: ["core", "oslo"] },
  { id: "sw2", name: "switch-2", type: "Switch", tags: ["edge", "bergen"] },
];

const mockSchedules: Schedule[] = [
  { id: "sch-1", name: "Nightly backup", cron: "0 2 * * *" },
];

export async function fetchInventory(): Promise<InventoryItem[]> {
  await delay(200);
  return mockInventory;
}

export async function fetchSchedules(): Promise<Schedule[]> {
  await delay(200);
  return mockSchedules;
}

export async function planJob(body: JobPlanRequest): Promise<JobPlanResponse> {
  await delay(400);
  const hasContent = body.content.trim().length > 0;
  if (!hasContent) {
    return { status: "error", message: "Commands or config snippet required." };
  }
  return {
    status: "planned",
    message: `Planned ${body.type} job "${body.name || "unnamed"}" for ${
      body.target || "all devices"
    }${body.dryRun ? " (dry run)" : ""}.`,
  };
}

export async function triggerComplianceSnapshot(): Promise<ComplianceSnapshot> {
  await delay(300);
  return { status: "running", message: "Snapshot started..." };
}
