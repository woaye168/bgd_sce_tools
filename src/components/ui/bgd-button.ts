/**
 * <bgd-button> 按钮
 * 属性：variant — primary（默认）/ secondary / danger；disabled
 * 事件：click（原生冒泡，disabled 时不触发）
 * 插槽：默认 slot — 按钮文字
 */
type ButtonVariant = "primary" | "secondary" | "danger";

const template = document.createElement("template");
template.innerHTML = `
  <style>
    :host {
      display: inline-block;
    }
    button {
      border: 1px solid transparent;
      border-radius: 0.5rem; /* rounded-lg */
      padding: 0.5rem 1rem; /* px-4 py-2 */
      font-size: 0.875rem;
      font-family: inherit;
      cursor: pointer;
      transition: background-color 0.15s ease, border-color 0.15s ease;
    }
    button:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
    button.variant-primary {
      background: #4f46e5; /* indigo-600 */
      color: #ffffff;
    }
    button.variant-primary:hover:not(:disabled) {
      background: #6366f1; /* indigo-500 */
    }
    button.variant-secondary {
      background: transparent;
      color: #e2e8f0; /* slate-200 */
      border-color: #475569; /* slate-600 */
    }
    button.variant-secondary:hover:not(:disabled) {
      background: #334155; /* slate-700 */
    }
    button.variant-danger {
      background: #dc2626; /* red-600 */
      color: #ffffff;
    }
    button.variant-danger:hover:not(:disabled) {
      background: #ef4444; /* red-500 */
    }
  </style>
  <button part="button" class="variant-primary" type="button">
    <slot></slot>
  </button>
`;

export class BgdButton extends HTMLElement {
  static get observedAttributes(): string[] {
    return ["variant", "disabled"];
  }

  private buttonEl: HTMLButtonElement;

  constructor() {
    super();
    const root = this.attachShadow({ mode: "open" });
    root.appendChild(template.content.cloneNode(true));
    this.buttonEl = root.querySelector("button") as HTMLButtonElement;
    this.buttonEl.addEventListener("click", (event) => {
      if (this.disabled) {
        event.stopImmediatePropagation();
        event.preventDefault();
      }
    });
  }

  connectedCallback(): void {
    this.render();
  }

  attributeChangedCallback(): void {
    this.render();
  }

  get variant(): ButtonVariant {
    const value = this.getAttribute("variant");
    if (value === "secondary" || value === "danger") {
      return value;
    }
    return "primary";
  }

  set variant(value: ButtonVariant) {
    this.setAttribute("variant", value);
  }

  get disabled(): boolean {
    return this.hasAttribute("disabled");
  }

  set disabled(value: boolean) {
    this.toggleAttribute("disabled", value);
  }

  private render(): void {
    this.buttonEl.className = `variant-${this.variant}`;
    this.buttonEl.disabled = this.disabled;
  }
}
