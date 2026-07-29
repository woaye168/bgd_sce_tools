/**
 * <bgd-select> 下拉选择
 * 属性：options — JSON 字符串，格式 [{ value: string, label: string }]；
 *       value — 当前选中值；disabled
 * 事件：change（detail: { value: string }，bubbles + composed）
 */
interface SelectOption {
  value: string;
  label: string;
}

const template = document.createElement("template");
template.innerHTML = `
  <style>
    :host {
      display: block;
    }
    select {
      width: 100%;
      box-sizing: border-box;
      border: 1px solid #475569; /* slate-600 */
      background: #1e293b; /* slate-800，option 下拉列表需要不透明底色 */
      color: #e2e8f0; /* slate-200 */
      border-radius: 0.5rem; /* rounded-lg */
      padding: 0.5rem 0.75rem; /* px-3 py-2 */
      font-size: 0.875rem;
      font-family: inherit;
      outline: none;
      cursor: pointer;
      transition: border-color 0.15s ease;
    }
    select:focus {
      border-color: #6366f1; /* indigo-500 */
    }
    select:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  </style>
  <select part="select"></select>
`;

export class BgdSelect extends HTMLElement {
  static get observedAttributes(): string[] {
    return ["options", "value", "disabled"];
  }

  private selectEl: HTMLSelectElement;

  constructor() {
    super();
    const root = this.attachShadow({ mode: "open" });
    root.appendChild(template.content.cloneNode(true));
    this.selectEl = root.querySelector("select") as HTMLSelectElement;
    this.selectEl.addEventListener("change", () => {
      this.setAttribute("value", this.selectEl.value);
      this.dispatchEvent(
        new CustomEvent("change", {
          detail: { value: this.selectEl.value },
          bubbles: true,
          composed: true,
        }),
      );
    });
  }

  connectedCallback(): void {
    this.render();
  }

  attributeChangedCallback(): void {
    this.render();
  }

  get options(): SelectOption[] {
    const raw = this.getAttribute("options");
    if (!raw) {
      return [];
    }
    try {
      const parsed: unknown = JSON.parse(raw);
      if (!Array.isArray(parsed)) {
        return [];
      }
      return parsed.filter(
        (item): item is SelectOption =>
          typeof item === "object" &&
          item !== null &&
          typeof (item as SelectOption).value === "string" &&
          typeof (item as SelectOption).label === "string",
      );
    } catch {
      return [];
    }
  }

  set options(value: SelectOption[]) {
    this.setAttribute("options", JSON.stringify(value));
  }

  get value(): string {
    return this.getAttribute("value") ?? "";
  }

  set value(value: string) {
    this.setAttribute("value", value);
  }

  get disabled(): boolean {
    return this.hasAttribute("disabled");
  }

  set disabled(value: boolean) {
    this.toggleAttribute("disabled", value);
  }

  private render(): void {
    const options = this.options;
    const current = this.value;
    this.selectEl.replaceChildren(
      ...options.map((option) => {
        const el = document.createElement("option");
        el.value = option.value;
        el.textContent = option.label;
        el.selected = option.value === current;
        return el;
      }),
    );
    this.selectEl.value = current;
    this.selectEl.disabled = this.disabled;
  }
}
