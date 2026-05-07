import { listen } from '@tauri-apps/api/event';
import type { LabEvent, SessionId } from './types';

export function onLabEvent(
  handler: (event: LabEvent) => void
): Promise<() => void> {
  return listen<LabEvent>('lab-event', (e) => {
    handler(e.payload);
  });
}

export function onSessionEvent(
  sessionId: SessionId,
  handler: (event: LabEvent) => void
): Promise<() => void> {
  return listen<LabEvent>('lab-event', (e) => {
    const event = e.payload;
    if ('session_id' in (event.payload as Record<string, unknown>)) {
      const payload = event.payload as { session_id: SessionId };
      if (payload.session_id === sessionId) {
        handler(event);
      }
    }
  });
}
