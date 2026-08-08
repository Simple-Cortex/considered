export function Violations() {
  const pending = fetch('/api/queues');

  return (
    <main>
      <h1>Overview</h1>
      <h1>Metrics</h1>
      <button type="button"><svg viewBox="0 0 16 16"><path d="M2 2h12" /></svg></button>
      <button type="button">Update</button>
      <Button variant="primary">Save one</Button>
      <Button variant="primary">Save two</Button>
      <Button variant="primary">Save three</Button>
      <span>🔥 🚀 🎯</span>
      <table><tbody><tr><td>42</td></tr></tbody></table>
      {pending}
    </main>
  );
}
