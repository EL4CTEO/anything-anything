import "./style.css";
import { roll, title } from "./app.ts";

const root = document.querySelector<HTMLElement>("#app");
if (root) {
  root.innerHTML = `<h1>${title()}</h1><button>roll</button><output></output>`;
  root.querySelector("button")?.addEventListener("click", () => {
    const out = root.querySelector("output");
    if (out) out.textContent = String(roll(6, Math.random));
  });
}
