import { mount } from "svelte";
import "../lib/styles/base.css";
import { restoreAppearance } from "../lib/ui/appearance-cache";
import Chat from "./Chat.svelte";

restoreAppearance();

export default mount(Chat, { target: document.getElementById("app")! });
