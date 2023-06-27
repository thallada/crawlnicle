import { Application } from "@hotwired/stimulus";

import LocalTimeController from "./local_time_controller";

window.Stimulus = Application.start();
window.Stimulus.register("local-time", LocalTimeController);
