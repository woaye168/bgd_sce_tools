/**
 * <bgd-tabs> 标签页
 * 属性：tabs — JSON 字符串，格式 [{ key: string, title: string }]；
 *       active — 当前激活标签的 key
 * 事件：change（detail: { key: string }，bubbles + composed）
 * 插槽：命名 slot，name 为对应 tab 的 key；未匹配的 slot 不显示。
 */
interface TabItem {
  key: string;
  title: string;
}

const template = document.createElement("template");
template.innerHTML = `
  <style>
    :host {
      display: block;
      font-family: inherit;
    }
    .tab-bar {
      display: flex;
      gap: 0.25rem;
      border-bottom: 1px solid #334155; /* slate-700 */
    }
    .tab {
      border: none;
      background: transparent;
      color: #94a3b8; /* slate-400 */
      font-size: 0.875rem;
      font-family: inherit;
      padding: 0.5rem 1rem;
      cursor: pointer;
      border-bottom: 2px solid transparent;
      margin-bottom: -1px;
      transition: color 0.15s ease, border-color 0.15s ease;
    }
    .tab:hover {
      color: #e2e8f0; /* slate-200 */
    }
    .tab.active {
      color: #818cf8; /* indigo-400 */
      border-bottom-color: #4f46e5; /* indigo-600 */
      font-weight: 600;
    }
    .panels {
      padding-top: 1rem;
      color: #e2e8f0;
    }
    ::slotted([slot]) {
      display: none;
    }
  </style>
  <div class="tab-bar" part="tab-bar"></div>
  <div class="panels" part="panels"></div>
`;

export class BgdTabs extends HTMLElement {
  static get observedAttributes(): string[] {
    return ["tabs", "active"];
  }

  private tabBarEl: HTMLDivElement;
  private panelsEl: HTMLDivElement;

  constructor() {
    super();
    const root = this.attachShadow({ mode: "open" });
    root.appendChild(template.content.cloneNode(true));
    this.tabBarEl = root.querySelector(".tab-bar") as HTMLDivElement;
    this.panelsEl = root.querySelector(".panels") as HTMLDivElement;
  }

  connectedCallback(): void {
    this.render();
  }

  attributeChangedCallback(): void {
    this.render();
  }

  get tabs(): TabItem[] {
    const raw = this.getAttribute("tabs");
    if (!raw) {
      return [];
    }
    try {
      const parsed: unknown = JSON.parse(raw);
      if (!Array.isArray(parsed)) {
        return [];
      }
      return parsed.filter(
        (item): item is TabItem =>
          typeof item === "object" &&
          item !== null &&
          typeof (item as TabItem).key === "string" &&
          typeof (item as TabItem).title === "string",
      );
    } catch {
      return [];
    }
  }

  set tabs(value: TabItem[]) {
    this.setAttribute("tabs", JSON.stringify(value));
  }

  get active(): string {
    return this.getAttribute("active") ?? this.tabs[0]?.key ?? "";
  }

  set active(value: string) {
    this.setAttribute("active", value);
  }

  private render(): void {
    const tabs = this.tabs;
    const active = this.active;

    this.tabBarEl.replaceChildren(
      ...tabs.map((tab) => {
        const button = document.createElement("button");
        button.type = "button";
        button.className = `tab${tab.key === active ? " active" : ""}`;
        button.textContent = tab.title;
        button.addEventListener("click", () => {
          if (tab.key === this.active) {
            return;
          }
          this.active = tab.key;
          this.dispatchEvent(
            new CustomEvent("change", {
              detail: { key: tab.key },
              bubbles: true,
              composed: true,
            }),
          );
        });
        return button;
      }),
    );

    this.panelsEl.replaceChildren(
      ...tabs.map((tab) => {
        const slot = document.createElement("slot");
        slot.name = tab.key;
        slot.style.display = tab.key === active ? "contents" : "none";
        return slot;
      }),
    );
  }
}
