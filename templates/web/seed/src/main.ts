// Starts {{name}} in the page. Yours: anything-anything never overwrites this file.
import "./style.css";
import { title } from "./app.ts";

const root = document.querySelector<HTMLElement>("#app");
if (root) {
  // ANYTHING:BUILD replace this with the real app
  root.innerHTML = `<h1>${title()}</h1><p>Not built yet: run anything-anything build.</p>`;
}
