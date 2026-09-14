// Placeholder-only fixture: only example.com addresses appear, which must
// never trigger an HON-03 lead.
export function renderInviteForm() {
  return `
    <label for="email">Work email</label>
    <input id="email" placeholder="you@example.com" />
    <p class="hint">We will never share this. Try name@example.com if unsure.</p>
  `;
}
