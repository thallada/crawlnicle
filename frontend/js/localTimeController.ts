function convertTimeElements() {
  const timeElements = document.querySelectorAll('time.local-time');
  timeElements.forEach((element) => {
    const utcString = element.getAttribute("datetime");
    if (utcString) {
      const utcTime = new Date(utcString);
      element.textContent = utcTime.toLocaleDateString(window.navigator.language, {
        year: "numeric",
        month: "long",
        day: "numeric",
      });
    } else {
      console.error("Missing datetime attribute on time.local-time element", element);
    }
  });
}

document.addEventListener("DOMContentLoaded", function() {
  convertTimeElements();
});

document.body.addEventListener('htmx:afterSwap', function() {
  convertTimeElements();
});
