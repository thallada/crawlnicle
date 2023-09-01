import htmx from 'htmx.org';
import 'htmx.org/dist/ext/sse';

// import CSS so it gets named with a content hash that busts caches
import '../css/styles.css';

import './localTimeController';

declare global {
  interface Window {
    htmx: typeof htmx;
  }
}

window.htmx = htmx;
