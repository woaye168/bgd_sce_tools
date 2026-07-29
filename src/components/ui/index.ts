/**
 * BGD UI Web Components —— 插件 iframe 通用组件库（零依赖，原生 Web Components）
 *
 * import 本模块即自动注册全部 customElements：
 *   import "bgd_sce_tools/ui"; // 或相对路径 import "./ui/index.js";
 */
import { BgdCard } from "./bgd-card";
import { BgdButton } from "./bgd-button";
import { BgdCheckbox } from "./bgd-checkbox";
import { BgdInput } from "./bgd-input";
import { BgdSelect } from "./bgd-select";
import { BgdTable } from "./bgd-table";
import { BgdTabs } from "./bgd-tabs";
import { BgdModal } from "./bgd-modal";

export { BgdCard, BgdButton, BgdCheckbox, BgdInput, BgdSelect, BgdTable, BgdTabs, BgdModal };

const registry: Array<[string, CustomElementConstructor]> = [
  ["bgd-card", BgdCard],
  ["bgd-button", BgdButton],
  ["bgd-checkbox", BgdCheckbox],
  ["bgd-input", BgdInput],
  ["bgd-select", BgdSelect],
  ["bgd-table", BgdTable],
  ["bgd-tabs", BgdTabs],
  ["bgd-modal", BgdModal],
];

for (const [name, ctor] of registry) {
  if (!customElements.get(name)) {
    customElements.define(name, ctor);
  }
}
