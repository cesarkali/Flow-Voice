import { mount } from "svelte";
import "../lib/styles/base.css";
import App from "./App.svelte";

export default mount(App, { target: document.getElementById("app")! });
