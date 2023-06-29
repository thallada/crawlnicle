import { Controller } from "@hotwired/stimulus";

// Replaces all UTC timestamps with time formated for the local timezone
export default class extends Controller {
  connect() {
    this.renderLocalTime();
  }

  renderLocalTime() {
    this.element.textContent = this.localTimeString;
  }

  get localTimeString(): string {
    if (this.utcTime) {
      return this.utcTime.toLocaleDateString(window.navigator.language, {
        year: "numeric",
        month: "long",
        day: "numeric",
      });
    }
    return "Unknown datetime"
  }

  get utcTime(): Date | null {
    const utcString = this.element.getAttribute("datetime");
    return utcString ? new Date(utcString) : null;
  }
}
