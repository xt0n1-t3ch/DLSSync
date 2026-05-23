import "./styles/global.css";
import "./styles/animations.css";
import { mount } from "svelte";
import App from "./App.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("app root not found");
}
const app = mount(App, { target });
export default app;
