import htmx from 'htmx.org';

declare global {
  interface Window {
    htmx: typeof htmx;
  }
}

window.htmx = htmx;

// Wait for htmx to be fully initialized before loading extensions
document.addEventListener(
  'htmx:load',
  () => {
    import('htmx.org/dist/ext/sse');
  },
  { once: true }
);

// import assets so they get named with a content hash that busts caches
// import '../css/styles.css';

import './localTimeController';
