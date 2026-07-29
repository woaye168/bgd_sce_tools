# BGD UI Web Components

供插件 iframe 使用的原生 Web Components 组件库。零依赖（不用 Lit），样式与 bgd_sce_tools 主界面一致（slate 暗色 + indigo 主色）。全部组件使用 Shadow DOM 隔离样式，可在任意页面直接使用。

## 引入

插件 iframe 是独立页面，直接引入打包产物（或源码模块）即可，import 后自动注册全部自定义元素：

```html
<!-- iframe 页面里 -->
<script type="module" src="/ui/index.js"></script>
```

或在插件自身的模块里：

```ts
import "bgd_sce_tools/ui"; // 按实际构建产物的路径引入
```

注册后即可在 HTML 中直接使用标签，也可以 `document.createElement("bgd-card")` 动态创建。

## 通用约定

- 布尔属性（`checked` / `disabled` / `open`）：存在即为 true，如 `<bgd-checkbox checked>`。
- JSON 属性（`options` / `columns` / `rows` / `tabs`）：值为 JSON 字符串，注意在 HTML 属性里用单引号包外层：`options='[{"value":"a","label":"A"}]'`。
- 所有自定义事件均 `bubbles + composed`，可穿透 Shadow DOM，在组件外层任意祖先节点监听。
- 每个属性都有同名 JS property，建议用 property 赋值对象/数组（如 `el.options = [...]`），避免手写 JSON 字符串。

## 组件一览

### `<bgd-card>` 卡片容器

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `title` | string | 卡片标题，为空则不渲染标题 |

内容放入默认插槽。

```html
<bgd-card title="构建设置">
  <p>卡片内容</p>
</bgd-card>
```

### `<bgd-button>` 按钮

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `variant` | `primary`（默认）/ `secondary` / `danger` | 按钮风格 |
| `disabled` | boolean | 禁用 |

事件：`click`（原生事件，disabled 时不触发）。

```html
<bgd-button variant="primary" id="buildBtn">开始构建</bgd-button>
<script>
  document.getElementById("buildBtn").addEventListener("click", () => { /* ... */ });
</script>
```

### `<bgd-checkbox>` 勾选框

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `label` | string | 文字 |
| `checked` | boolean | 是否勾选 |
| `disabled` | boolean | 禁用 |

事件：`change`，`event.detail = { checked: boolean }`。

```html
<bgd-checkbox label="构建后自动清理日志" checked></bgd-checkbox>
```

### `<bgd-input>` 输入框

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `placeholder` | string | 占位文字 |
| `value` | string | 当前值 |
| `type` | string | 原生 input type，默认 `text` |
| `disabled` | boolean | 禁用 |

事件：`input`，`event.detail = { value: string }`。

```html
<bgd-input placeholder="项目路径"></bgd-input>
```

### `<bgd-select>` 下拉选择

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `options` | JSON | `[{ "value": "a", "label": "选项 A" }]` |
| `value` | string | 当前选中值 |
| `disabled` | boolean | 禁用 |

事件：`change`，`event.detail = { value: string }`。

```html
<bgd-select id="mode"></bgd-select>
<script>
  const el = document.getElementById("mode");
  el.options = [
    { value: "build", label: "全量构建" },
    { value: "watch", label: "监听更新" },
  ];
  el.value = "build";
  el.addEventListener("change", (e) => console.log(e.detail.value));
</script>
```

### `<bgd-table>` 表格

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `columns` | JSON | `[{ "key": "name", "title": "名称", "sortable": true }]` |
| `rows` | JSON | `[{ "name": "...", ... }]`，按 columns 的 key 取值 |
| `sort-key` | string | 当前排序列（用于表头箭头展示） |
| `sort-order` | `asc` / `desc` | 当前排序方向 |

事件：`sort`（点击 sortable 表头），`event.detail = { key, order }`。

> 组件只负责展示和抛出排序意图，**不主动排序数据**。使用者在 `sort` 事件里排序 rows 后重新赋值，并同步设置 `sort-key` / `sort-order`。

```html
<bgd-table id="logTable"></bgd-table>
<script>
  const table = document.getElementById("logTable");
  table.columns = [
    { key: "time", title: "时间", sortable: true },
    { key: "message", title: "内容" },
  ];
  table.rows = [{ time: "12:00", message: "构建完成" }];
  table.addEventListener("sort", (e) => {
    const { key, order } = e.detail;
    table.setAttribute("sort-key", key);
    table.setAttribute("sort-order", order);
    // 自行排序后：table.rows = sorted;
  });
</script>
```

### `<bgd-tabs>` 标签页

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `tabs` | JSON | `[{ "key": "build", "title": "构建" }]` |
| `active` | string | 当前激活标签 key，默认第一个 |

事件：`change`，`event.detail = { key: string }`。

每个标签的内容放在**命名插槽**里（`slot="<key>"`），只显示 active 对应的面板。

```html
<bgd-tabs id="mainTabs" active="build">
  <div slot="build">构建面板内容</div>
  <div slot="watch">监听面板内容</div>
</bgd-tabs>
<script>
  const tabs = document.getElementById("mainTabs");
  tabs.tabs = [
    { key: "build", title: "构建" },
    { key: "watch", title: "监听" },
  ];
  tabs.addEventListener("change", (e) => console.log(e.detail.key));
</script>
```

### `<bgd-modal>` 弹窗

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `open` | boolean | 是否打开 |
| `title` | string | 标题 |

事件：`close`（点击遮罩 / 右上角 × / Esc）。组件不自动关闭，使用者在 `close` 事件里移除 `open` 属性。

插槽：默认插槽为内容；`slot="footer"` 为底部操作区（无内容时底部不渲染）。

```html
<bgd-button id="openBtn">打开</bgd-button>
<bgd-modal id="dlg" title="确认操作">
  <p>确定要执行全量构建吗？</p>
  <bgd-button slot="footer" variant="secondary" id="cancelBtn">取消</bgd-button>
  <bgd-button slot="footer" id="okBtn">确定</bgd-button>
</bgd-modal>
<script>
  const dlg = document.getElementById("dlg");
  document.getElementById("openBtn").addEventListener("click", () => (dlg.open = true));
  dlg.addEventListener("close", () => (dlg.open = false));
  document.getElementById("cancelBtn").addEventListener("click", () => (dlg.open = false));
</script>
```

## 样式说明

组件样式写在各自 Shadow DOM 内（不依赖 Tailwind 运行时），色值与主界面 Tailwind 类一一对应：

| 用途 | 色值 | 对应 Tailwind |
| --- | --- | --- |
| 卡片/弹窗背景 | `#1e293b` | `bg-slate-800` |
| 边框 | `#334155` / `#475569` | `border-slate-700` / `border-slate-600` |
| 主文字 | `#e2e8f0` | `text-slate-200` |
| 次文字 | `#94a3b8` | `text-slate-400` |
| 主按钮 | `#4f46e5` / hover `#6366f1` | `bg-indigo-600` / `hover:bg-indigo-500` |
| 危险按钮 | `#dc2626` / hover `#ef4444` | `bg-red-600` / `hover:bg-red-500` |
| 圆角 | `0.5rem` / `0.75rem` | `rounded-lg` / `rounded-xl` |

iframe 页面本身建议设置深色底色，与主界面融合：

```css
body { background: #0f172a; color: #e2e8f0; font-family: system-ui, sans-serif; margin: 0; padding: 1rem; }
```
