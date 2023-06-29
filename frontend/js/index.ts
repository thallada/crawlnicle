import { Application } from "@hotwired/stimulus";

import LocalTimeController from "./local_time_controller";

// import CSS so it gets named with a content hash that busts caches
import "../css/styles.css";

window.Stimulus = Application.start();
window.Stimulus.register("local-time", LocalTimeController);
