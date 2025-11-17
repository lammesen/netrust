# Frontend Implementation Plan

## Stack
- React + Vite + TypeScript + Tailwind.
- UI kit: shadcn/ui for defaults (Radix-based) plus selected Headless UI pieces (Combobox/Listbox/Transition) with shared Tailwind tokens for consistency.
- State/data: TanStack Query for server state; React Hook Form + Zod for forms/validation.
- Routing/layout: Single-page shell or simple routes in Vite; no heavy router needed initially.
- Icons: Lucide.
- Styling: Tailwind theme tokens (colors, radii, shadows), dark-first with optional light mode later.

## Progress
- ✅ Vite + React + TS scaffolded with Tailwind, shadcn init, Headless UI, TanStack Query, RHF, Zod, clsx/twMerge utilities.
- ✅ Tailwind theme configured with dark-first palette, CSS vars, fonts, radius, shadows; base styles wired.
- ✅ shadcn components generated (button, input, textarea, select, checkbox, switch, card, table, form, badge, tooltip, skeleton, toast, alert, scroll-area) for reuse.
- ✅ Layout rebuilt with shadcn components matching the mock (header + Inventory/Job Wizard/Scheduling/Compliance cards).
- ✅ QueryClient wired in app root with Toaster mounted.
- ✅ RHF + Zod wiring on Job Wizard and Scheduling forms with validation and loading/disabled states; toasts on job plan and schedules.
- ✅ Mock data layer + TanStack Query hooks for inventory, schedules, job planning mutation, and compliance snapshot mutation.

## Phased Plan
1) Scaffold (done)
   - Create Vite React TS app. ✅
   - Add Tailwind, shadcn/ui init, Headless UI, TanStack Query, RHF, Zod, clsx/twMerge. ✅
2) Theming (done)
   - Tailwind config with dark palette matching mock (navy/charcoal surfaces, cyan CTA, subtle borders/shadows). ✅
   - CSS vars for semantic colors (surface, surface-2, border, accent, text). ✅
   - Configure fonts, base rounded corners, shadows, focus rings. ✅
3) Layout Shell (done)
   - App container with header bar (title, status indicators). ✅
   - Responsive grid for panels: Inventory, Job Wizard, Scheduling, Compliance. ✅
   - Consistent Card component (shadcn Card with custom styling). ✅
4) Reusable Atoms (done)
   - Button variants (primary, ghost, danger), Input, Textarea, Select, Checkbox/Switch, Tabs, Badge, Tooltip, Skeleton, Alert/Callout, Loader. ✅ generated via shadcn
   - Style once via shadcn; expose className slots for Headless UI pieces. ✅ base utilities ready
5) Forms (done for initial screens)
   - Form wrapper using RHF + Zod + shadcn Form primitives. ✅ applied to Job Wizard + Scheduling
   - Helpers: FieldLabel, Description, Error, InlineHelp. ⏳ optional niceties
   - Submit/loading/disabled states baked in. ✅
6) Data Layer (partial)
   - QueryClient setup; API client (fetch/axios) with typed endpoints. ✅ mock API + hooks in place
   - Polling helpers for long-running jobs. ⏳
   - Centralized error handling and toast notifications. ⏳ toasts partially wired
7) Inventory Panel
   - Table using TanStack Table + shadcn styling for sorting/filtering/search.
   - Empty/loading/error states; row actions (view device/run job) placeholders.
8) Job Wizard
   - Single form or accordion sections.
   - Fields: job name, type (select), target filter (combobox/autocomplete), commands/snippet (textarea or code editor).
   - Dry run toggle; submit button with progress; status log area.
9) Scheduling
   - Cron input with helper presets (Headless UI Combobox/Listbox) + validation (zod + cron parser).
   - List of schedules (table/cards) with enable/disable/delete actions; show next run if available.
10) Compliance
   - Refresh snapshot action; polling status; display last updated time and summary/log.
11) Feedback UX
   - Toasts for success/failure; inline errors; optimistic updates for small mutations.
   - Skeletons/spinners for load states.
12) Accessibility
   - Use Headless UI/Radix defaults; clear focus rings, aria labels, keyboard navigation, reduced motion on transitions.
13) Testing
   - Vitest + React Testing Library for components.
   - Optional Playwright smoke; zod schemas as contract tests for API shapes.

## Component/Lib Map
- Layout & cards: shadcn Card, Tabs.
- Buttons/inputs: shadcn Button/Input/Select/Checkbox/Switch/Textarea.
- Combobox/autocomplete: Headless UI Combobox styled with Tailwind tokens.
- Tables: TanStack Table + shadcn wrappers for Inventory/Schedules.
- Dialogs/overlays: shadcn Dialog/Popover/Tooltip for confirmations/helpers.
- Logs/output: Pre-styled `<pre>` component; optional Monaco/CodeMirror if needed later.
- Toasts: shadcn Toast system.
- Cron help: cron-parser + Headless UI Listbox/Combobox for presets.
- Load states: shadcn Skeleton/Spinner.

## Feature Flow Notes
- Inventory: fetch list; text filter; badges for type/tags; empty/error messaging.
- Job Wizard: RHF + Zod; validate required fields; mutation with spinner; show server plan/result in log box; dry-run flag; type select drives textarea placeholder.
- Scheduling: add schedule form; validate cron; list existing schedules; enable/disable/delete; surface next run.
- Compliance: trigger snapshot; poll status; show last updated + log/summary.

## Immediate Deliverables
- ✅ Tailwind theme config and global styles.
- ✅ shadcn init with tokens/utilities; core components generated.
- ✅ Layout skeleton with cards and placeholder data.
- ✅ QueryClient provider + Toaster mounted.
- ✅ Form primitives (RHF + shadcn) on Job Wizard & Scheduling.
- ✅ Data layer stubs (mock API + TanStack Query hooks).
- 🔜 Polling helpers and richer status handling for jobs/compliance; table wrappers for inventory/schedules.
