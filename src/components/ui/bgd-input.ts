/**
 * <bgd-input> 输入框
 * 属性：placeholder / value / disabled / type（默认 text）
 * 事件：input（detail: { value: string }，bubbles + composed）
 */
const template = document.createElement("template");
template.innerHTML = `
  <style>
    :host {
      display: block;
    }
    input {
      width: 100%;
      box-sizing: border-box;
      border: 1px solid #475569; /* slate-600 */
      background: transparent;
      color: #e2e8f0; /* slate-200 */
      border-radius: 0.5rem; /* rounded-lg */
      padding: 0.5rem 0.75rem; /* px-3 py-2 */
      font-size: 0.875rem;
      font-family: inherit;
      outline: none;
      transition: border-color 0.15s ease;
    }
    input::placeholder {
      color: #64748b; /* slate-500 */
    }
    input:focus {
      border-color: #6366f1; /* indigo-500 */
    }
    input:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  </style>
  <input part="input" />
`;

export class BgdInput extends HTMLElement {
  static get observedAttributes(): string[] {
    return ["placeholder", "value", "disabled", "type"];
  }

  private inputEl: HTMLInputElement;

  constructor() {
    super();
    const root = this.attachShadow({ mode: "open" });
    root.appendChild(template.content.cloneNode(true));
    this.inputEl = root.querySelector("input") as HTMLInputElement;
    this.inputEl.addEventListener("input", () => {
      this.dispatchEvent(
        new CustomEvent("input", {
          detail: { value: this.inputEl.value },
          bubbles: true,
          composed: true,
        }),
      );
    });
  }

  connectedCallback(): void {
    this.render();
  }

  attributeChangedCallback(name: string, oldValue: string | null, newValue: string | null): void {
    if (oldValue === newValue) {
      return;
    }
    if (name === "value" && this.inputEl.value !== (newValue ?? "")) {
      this.inputEl.value = newValue ?? "";
      return;
    }
    this.render();
  }

  get value(): string {
    return this.inputEl.value;
  }

  set value(value: string) {
    this.inputEl.value = value;
    this.setAttribute("value", value);
  }

  get placeholder(): string {
    return this.getAttribute("placeholder") ?? "";
  }

  set placeholder(value: string) {
    this.setAttribute("placeholder", value);
  }

  get disabled(): boolean {
    return this.hasAttribute("disabled");
  }

  set disabled(value: boolean) {
    this.toggleAttribute("disabled", value);
  }

  private render(): void {
    this.inputEl.placeholder = this.placeholder;
    this.inputEl.value = this.getAttribute("value") ?? "";
    this.inputEl.disabled = this.disabled;
    this.inputEl.type = this.getAttribute("type") ?? "text";
  }
}
