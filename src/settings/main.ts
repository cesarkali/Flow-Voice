import { mount } from "svelte";
import "../lib/styles/base.css";
import { restoreAppearance } from "../lib/ui/appearance-cache";
import App from "./App.svelte";

restoreAppearance();

export default mount(App, { target: document.getElementById("app")! });
