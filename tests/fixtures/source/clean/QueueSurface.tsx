export function QueueSurface() {
  return (
    <main className="queue-surface">
      <h1>Support queue</h1>
      <p>Which queue needs an agent next?</p>
      <button type="button">Reassign agent</button>
      <button type="button" aria-label="Open queue filters">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M2 3h12" /></svg>
      </button>
    </main>
  );
}
