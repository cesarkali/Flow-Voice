import { mount } from "svelte";
import "../lib/styles/base.css";
import { isTauri } from "../lib/api";
import Overlay from "./Overlay.svelte";

async function start() {
  const target = document.getElementById("app")!;
  if (isTauri) return mount(Overlay, { target });
  // Plain browser: show the overlay over a fake desktop with scenario controls.
  const { default: Preview } = await import("./Preview.svelte");
  return mount(Preview, { target });
}

export default start();
