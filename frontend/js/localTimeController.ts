// This file is used to convert UTC timestamps from the server to human-readable local time in the browser.
//
// Usage: Add a time element with a `datetime` attribute and a `data-local-time` attribute to the HTML.
//
// `data-local-time` can be either 'relative' or 'date'.
// 'relative' will show the time in a human-readable format, e.g. "2 hours from now".
// 'date' will show the date in a human-readable format, e.g. "January 1, 2022".
// Any other value will be ignored and time will be shown as-is from the server as a UTC timestamp.

function pluralize(count: number, singular: string, plural: string): string {
  return count === 1 ? singular : plural;
}

function formatRelativeTime(utcTime: Date): string {
  const now = new Date();
  const diffInSeconds = (utcTime.getTime() - now.getTime()) / 1000;

  if (diffInSeconds < -86400) {
    const days = Math.round(Math.abs(diffInSeconds) / 86400);
    return `${days} ${pluralize(days, 'day', 'days')} ago`;
  } else if (diffInSeconds < -3600) {
    const hours = Math.round(Math.abs(diffInSeconds) / 3600);
    return `${hours} ${pluralize(Math.abs(hours), 'hour', 'hours')} ago`;
  } else if (diffInSeconds < -60) {
    const minutes = Math.round(Math.abs(diffInSeconds) / 60);
    return `${minutes} ${pluralize(minutes, 'minute', 'minutes')} ago`;
  } else if (diffInSeconds < 0) {
    const seconds = Math.abs(diffInSeconds);
    return `${seconds} ${pluralize(seconds, 'second', 'seconds')} ago`;
  } else if (diffInSeconds < 60) {
    return `${diffInSeconds} ${pluralize(
      diffInSeconds,
      'second',
      'seconds'
    )} from now`;
  } else if (diffInSeconds < 3600) {
    const minutes = Math.round(diffInSeconds / 60);
    return `${minutes} ${pluralize(minutes, 'minute', 'minutes')} from now`;
  } else if (diffInSeconds < 86400) {
    const hours = Math.round(diffInSeconds / 3600);
    return `${hours} ${pluralize(hours, 'hour', 'hours')} from now`;
  } else {
    const days = Math.round(diffInSeconds / 86400);
    return `${days} ${pluralize(days, 'day', 'days')} from now`;
  }
}

function convertTimeElements(): void {
  const timeElements = document.querySelectorAll('time[data-local-time]');
  timeElements.forEach((element) => {
    const utcString = element.getAttribute('datetime');
    if (utcString !== null) {
      const utcTime = new Date(utcString);
      const localTimeType = element.getAttribute('data-local-time');
      if (localTimeType === 'relative') {
        element.textContent = formatRelativeTime(utcTime);
      } else if (localTimeType === 'date') {
        element.textContent = utcTime.toLocaleDateString(
          window.navigator.language,
          {
            year: 'numeric',
            month: 'long',
            day: 'numeric',
          }
        );
      } else {
        console.error(
          'Unrecognized data-local-time attribute value on time[data-local-time] element. Local time cannot be displayed.',
          element
        );
      }
    } else {
      console.error(
        'Missing datetime attribute on time[data-local-time] element',
        element
      );
    }
  });
}

document.addEventListener('DOMContentLoaded', function () {
  convertTimeElements();
});

document.body.addEventListener('htmx:afterSwap', function () {
  convertTimeElements();
});
