// Negative fixture: none of these should ever produce an HON-03 lead.
// Import specifiers look like IANA timezone segments; identifiers contain
// "card" as a substring without naming a card/last-four field.
import { Button } from "Components/Button";
import { formatDate } from "Utils/DateFormat";
import Modal from "Overlay/Modal";

const cardWidth = 1280;
const wildcardTimeout = 3000;
const dashboardHeight = 1024;

export default function Footer() {
  return (
    <footer>
      <span>Contact the team for details.</span>
    </footer>
  );
}
