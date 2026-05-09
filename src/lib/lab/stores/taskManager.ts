import { writable, derived, get } from 'svelte/store';
import { i18n } from '$lib/i18n';

export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface BackgroundTask {
  id: string;
  name: string;
  description: string;
  status: TaskStatus;
  progress: number;
  progressMessage: string;
  startedAt: number;
  completedAt: number | null;
  estimatedTimeRemaining: number | null;
  cancellable: boolean;
  result: string | null;
  error: string | null;
  currentStep: number;
  totalSteps: number;
  stepLabel: string;
  progressHistory: { timestamp: number; progress: number }[];
}

function createTaskManagerStore() {
  const { subscribe, update } = writable<BackgroundTask[]>([]);
  let nextId = 0;

  function createTask(
    name: string,
    description: string,
    cancellable: boolean = false,
    totalSteps: number = 0,
  ): string {
    const id = `task_${nextId++}`;
    const task: BackgroundTask = {
      id,
      name,
      description,
      status: 'running',
      progress: 0,
      progressMessage: get(i18n.t)('taskManager.preparing'),
      startedAt: Date.now(),
      completedAt: null,
      estimatedTimeRemaining: null,
      cancellable,
      result: null,
      error: null,
      currentStep: 0,
      totalSteps,
      stepLabel: '',
      progressHistory: [{ timestamp: Date.now(), progress: 0 }],
    };
    update((tasks) => [...tasks, task]);
    return id;
  }

  function updateProgress(
    taskId: string,
    progress: number,
    message: string,
    estimatedTimeRemaining?: number | null,
  ) {
    update((tasks) =>
      tasks.map((t) => {
        if (t.id !== taskId) return t;
        const now = Date.now();
        const history = [...t.progressHistory, { timestamp: now, progress }];
        if (history.length > 20) history.shift();
        let eta = estimatedTimeRemaining !== undefined ? estimatedTimeRemaining : t.estimatedTimeRemaining;
        if (eta === undefined || eta === null) {
          eta = computeETA(t.startedAt, progress, history);
        }
        return {
          ...t,
          progress: Math.min(100, Math.max(0, progress)),
          progressMessage: message,
          estimatedTimeRemaining: eta,
          progressHistory: history,
        };
      }),
    );
  }

  function updateStep(
    taskId: string,
    currentStep: number,
    totalSteps: number,
    stepLabel: string,
  ) {
    update((tasks) =>
      tasks.map((t) => {
        if (t.id !== taskId) return t;
        const progress = totalSteps > 0 ? Math.round((currentStep / totalSteps) * 100) : t.progress;
        const now = Date.now();
        const history = [...t.progressHistory, { timestamp: now, progress }];
        if (history.length > 20) history.shift();
        const eta = computeETA(t.startedAt, progress, history);
        return {
          ...t,
          currentStep,
          totalSteps,
          stepLabel,
          progress,
          progressMessage: stepLabel,
          estimatedTimeRemaining: eta,
          progressHistory: history,
        };
      }),
    );
  }

  function advanceStep(taskId: string, stepLabel: string) {
    update((tasks) =>
      tasks.map((t) => {
        if (t.id !== taskId) return t;
        const newStep = t.currentStep + 1;
        const progress = t.totalSteps > 0 ? Math.round((newStep / t.totalSteps) * 100) : t.progress;
        const now = Date.now();
        const history = [...t.progressHistory, { timestamp: now, progress }];
        if (history.length > 20) history.shift();
        const eta = computeETA(t.startedAt, progress, history);
        return {
          ...t,
          currentStep: newStep,
          stepLabel,
          progress,
          progressMessage: stepLabel,
          estimatedTimeRemaining: eta,
          progressHistory: history,
        };
      }),
    );
  }

  function completeTask(taskId: string, result?: string) {
    update((tasks) =>
      tasks.map((t) =>
        t.id === taskId
          ? {
              ...t,
              status: 'completed' as const,
              progress: 100,
              progressMessage: get(i18n.t)('taskManager.completed'),
              completedAt: Date.now(),
              estimatedTimeRemaining: null,
              result: result || null,
              currentStep: t.totalSteps > 0 ? t.totalSteps : t.currentStep,
            }
          : t,
      ),
    );
    scheduleCleanup(taskId, 8000);
  }

  function failTask(taskId: string, error: string) {
    update((tasks) =>
      tasks.map((t) =>
        t.id === taskId
          ? {
              ...t,
              status: 'failed' as const,
              progress: t.progress,
              progressMessage: get(i18n.t)('taskManager.failed'),
              completedAt: Date.now(),
              estimatedTimeRemaining: null,
              error,
            }
          : t,
      ),
    );
    scheduleCleanup(taskId, 15000);
  }

  function cancelTask(taskId: string) {
    update((tasks) =>
      tasks.map((t) =>
        t.id === taskId
          ? {
              ...t,
              status: 'cancelled' as const,
              progress: t.progress,
              progressMessage: get(i18n.t)('taskManager.cancelled'),
              completedAt: Date.now(),
              estimatedTimeRemaining: null,
            }
          : t,
      ),
    );
    scheduleCleanup(taskId, 3000);
  }

  function removeTask(taskId: string) {
    update((tasks) => tasks.filter((t) => t.id !== taskId));
  }

  function scheduleCleanup(taskId: string, delay: number) {
    setTimeout(() => {
      update((tasks) => tasks.filter((t) => t.id !== taskId));
    }, delay);
  }

  function clearCompleted() {
    update((tasks) =>
      tasks.filter((t) => t.status === 'running' || t.status === 'pending'),
    );
  }

  return {
    subscribe,
    createTask,
    updateProgress,
    updateStep,
    advanceStep,
    completeTask,
    failTask,
    cancelTask,
    removeTask,
    clearCompleted,
  };
}

function computeETA(startedAt: number, currentProgress: number, history: { timestamp: number; progress: number }[]): number | null {
  if (currentProgress <= 0 || currentProgress >= 100) return null;
  const elapsed = Date.now() - startedAt;
  if (elapsed < 500) return null;
  const rate = currentProgress / elapsed;
  const remaining = (100 - currentProgress) / rate;
  if (remaining < 1000) return null;
  return Math.round(remaining);
}

export const taskManagerStore = createTaskManagerStore();

export const activeTaskCount = derived(taskManagerStore, ($tasks) =>
  $tasks.filter((t) => t.status === 'running' || t.status === 'pending').length,
);

export const hasActiveTasks = derived(activeTaskCount, ($count) => $count > 0);

export function formatETA(ms: number | null): string {
  if (ms === null) return '';
  const t = get(i18n.t);
  if (ms < 1000) return t('taskManager.almostDone');
  const seconds = Math.round(ms / 1000);
  if (seconds < 60) return t('taskManager.aboutSeconds', { seconds });
  const minutes = Math.floor(seconds / 60);
  const remainSeconds = seconds % 60;
  if (minutes < 60) return t('taskManager.aboutMinSec', { minutes, seconds: remainSeconds });
  const hours = Math.floor(minutes / 60);
  const remainMinutes = minutes % 60;
  return t('taskManager.aboutHourMin', { hours, minutes: remainMinutes });
}

export function formatElapsed(startedAt: number, completedAt: number | null): string {
  const end = completedAt || Date.now();
  const elapsed = end - startedAt;
  if (elapsed < 1000) return `${elapsed}ms`;
  const t = get(i18n.t);
  const seconds = Math.round(elapsed / 1000);
  if (seconds < 60) return t('taskManager.secondsOnly', { seconds });
  const minutes = Math.floor(seconds / 60);
  const remainSeconds = seconds % 60;
  return t('taskManager.minSec', { minutes, seconds: remainSeconds });
}
