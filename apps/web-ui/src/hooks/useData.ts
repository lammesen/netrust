import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  fetchInventory,
  fetchSchedules,
  planJob,
  triggerComplianceSnapshot,
} from "@/lib/api";
import type { JobPlanRequest } from "@/lib/api";

export function useInventory() {
  return useQuery({
    queryKey: ["inventory"],
    queryFn: fetchInventory,
  });
}

export function useSchedules() {
  return useQuery({
    queryKey: ["schedules"],
    queryFn: fetchSchedules,
  });
}

export function usePlanJob() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: JobPlanRequest) => planJob(body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["schedules"] });
    },
  });
}

export function useComplianceSnapshot() {
  return useMutation({
    mutationFn: triggerComplianceSnapshot,
  });
}
