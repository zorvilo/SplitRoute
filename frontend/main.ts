/**
 * 前端入口，加载统一样式并挂载应用组件。
 *
 * @author lhy
 */
import { createApp } from "vue";
import App from "./App.vue";
import "./styles.css";
createApp(App).mount("#app");
