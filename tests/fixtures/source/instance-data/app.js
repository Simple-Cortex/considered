// Settings surface fixture: every HON-03 instance-data class appears here
// unmarked, to prove the lint lead fires for each class.
export function renderAccountPanel() {
  return `
    <div class="member-row">Priya Chen priya@northwindresearch.com</div>
    <div class="member-row">Marcus Reyes marcus@brightpathstudio.io</div>
    <div class="billing-card">Card ending in 4242</div>
    <div class="invoice-row">Invoice — March 2024</div>
    <div class="token-row">last used 3 hours ago</div>
    <div class="timezone-value">America/New_York</div>
    <div class="hours-value">09:00 - 17:00</div>
    <div class="format-value">MM/DD/YYYY</div>
    <div class="locale-value">English (US)</div>
  `;
}
