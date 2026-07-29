/**
 * <bgd-checkbox> 勾选框
 * 属性：label — 文字；checked — 是否勾选；disabled
 * 事件：change（detail: { checked: boolean }，bubbles + composed）
 */
const template = document.createElement("template");
template.innerHTML = `
  <style>
    :host {
      display: inline-block;
    }
    label {
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      cursor: pointer;
      color: #e2e8f0; /* slate-200 */
      font-size: 0.875rem;
      font-family: inherit;
      user-select: none;
    }
    label.disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
    input {
      appearance: none;
      width: 1rem;
      height: 1rem;
      margin: 0;
      border: 1px solid #475569; /* slate-600 */
      border-radius: 0.25rem;
      background: transparent;
      cursor: inherit;
      position: relative;
      flex-shrink: 0;
    }
    input:checked {
      background: #4f46e5; /* indigo-600 */
      border-color: #4f46e5;
    }
    input:checked::after {
      content: "";
      position: absolute;
      left: 0.28rem;
      top: 0.06rem;
      width: 0.28rem;
      height: 0.55rem;
      border: solid #ffffff;
      border-width: 0 2px 2px 0;
      transform: rotate(45deg);
    }
  </style>
  <label part="label">
    <input type="checkbox" part="input" />
    <span class="text"></span>
  </label>
`;

export class BgdCheckbox extends HTMLElement {
  static get observedAttributes(): string[] {
    return ["label", "checked", "disabled"];
  }

  private inputEl: HTMLInputElement;
  private textEl: HTMLSpanElement;
  private labelEl: HTMLLabelElement;

  constructor() {
    super();
    const root = this.attachShadow({ mode: "open" });
    root.appendChild(template.content.cloneNode(true));
    this.inputEl = root.querySelector("input") as HTMLInputElement;
    this.textEl = root.querySelector(".text") as HTMLSpanElement;
    this.labelEl = root.querySelector("label") as HTMLLabelElement;
    this.inputEl.addEventListener("change", () => {
      this.toggleAttribute("checked", this.inputEl.checked);
      this.dispatchEvent(
        new CustomEvent("change", {
          detail: { checked: this.inputEl.checked },
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

  get label(): string {
    return this.getAttribute("label") ?? "";
  }

  set label(value: string) {
    this.setAttribute("label", value);
  }

  get checked(): boolean {
    return this.hasAttribute("checked");
  }

  set checked(value: boolean) {
    this.toggleAttribute("checked", value);
  }

  get disabled(): boolean {
    return this.hasAttribute("disabled");
  }

  set disabled(value: boolean) {
    this.toggleAttribute("disabled", value);
  }

  private render(): void {
    this.textEl.textContent = this.label;
    this.inputEl.checked = this.checked;
    this.inputEl.disabled = this.disabled;
    this.labelEl.classList.toggle("disabled", this.disabled);
  }
}
