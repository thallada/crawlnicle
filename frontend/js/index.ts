import htmx from 'htmx.org';

// import assets so they get named with a content hash that busts caches
// import '../css/styles.css';

import './localTimeController';

declare global {
  interface Window {
    htmx: typeof htmx;
  }
}

window.htmx = htmx;

// eslint-disable-next-line import/first
import 'htmx.org/dist/ext/sse';
