import { mount } from "svelte";
import "../lib/styles/base.css";
import Chat from "./Chat.svelte";

export default mount(Chat, { target: document.getElementById("app")! });
