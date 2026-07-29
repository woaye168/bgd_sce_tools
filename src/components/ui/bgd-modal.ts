/**
 * <bgd-modal> 弹窗
 * 属性：open — 是否打开；title — 标题
 * 事件：close（点击遮罩 / 关闭按钮 / Esc 时触发，bubbles + composed）
 * 插槽：默认 slot — 内容；slot="footer" — 底部操作区
 */
const template = document.createElement("template");
template.innerHTML = `
  <style>
    :host {
      display: none;
    }
    :host([open]) {
      display: block;
    }
    .overlay {
      position: fixed;
      inset: 0;
      background: rgba(2, 6, 23, 0.7); /* slate-950 / 70% */
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 1000;
      padding: 1rem;
      box-sizing: border-box;
    }
    .dialog {
      background: #1e293b; /* slate-800 */
      border: 1px solid #334155; /* slate-700 */
      border-radius: 0.75rem; /* rounded-xl */
      color: #e2e8f0; /* slate-200 */
      font-family: inherit;
      width: 100%;
      max-width: 32rem;
      max-height: calc(100vh - 4rem);
      display: flex;
      flex-direction: column;
      box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.4);
    }
    .header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 1rem 1.25rem;
      border-bottom: 1px solid #334155;
    }
    .header h2 {
      margin: 0;
      font-size: 1rem;
      font-weight: 600;
      color: #f1f5f9; /* slate-100 */
    }
    .close-btn {
      border: none;
      background: transparent;
      color: #94a3b8; /* slate-400 */
      font-size: 1.25rem;
      line-height: 1;
      cursor: pointer;
      padding: 0.25rem;
      border-radius: 0.375rem;
    }
    .close-btn:hover {
      color: #e2e8f0;
      background: #334155;
    }
    .body {
      padding: 1.25rem;
      overflow-y: auto;
      flex: 1;
    }
    .footer {
      padding: 0.75rem 1.25rem;
      border-top: 1px solid #334155;
      display: flex;
      justify-content: flex-end;
      gap: 0.5rem;
    }
    .footer[hidden] {
      display: none;
    }
  </style>
  <div class="overlay" part="overlay">
    <div class="dialog" part="dialog" role="dialog" aria-modal="true">
      <div class="header" part="header">
        <h2 class="title"></h2>
        <button class="close-btn" type="button" part="close-button" aria-label="关闭">×</button>
      </div>
      <div class="body" part="body">
        <slot></slot>
      </div>
      <div class="footer" part="footer">
        <slot name="footer"></slot>
      </div>
    </div>
  </div>
`;

export class BgdModal extends HTMLElement {
  static get observedAttributes(): string[] {
    return ["open", "title"];
  }

  private titleEl: HTMLHeadingElement;
  private overlayEl: HTMLDivElement;
  private dialogEl: HTMLDivElement;
  private footerEl: HTMLDivElement;
  private footerSlotEl: HTMLSlotElement;

  constructor() {
    super();
    const root = this.attachShadow({ mode: "open" });
    root.appendChild(template.content.cloneNode(true));
    this.titleEl = root.querySelector(".title") as HTMLHeadingElement;
    this.overlayEl = root.querySelector(".overlay") as HTMLDivElement;
    this.dialogEl = root.querySelector(".dialog") as HTMLDivElement;
    this.footerEl = root.querySelector(".footer") as HTMLDivElement;
    this.footerSlotEl = root.querySelector('slot[name="footer"]') as HTMLSlotElement;

    root.querySelector(".close-btn")?.addEventListener("click", () => this.requestClose());
    this.overlayEl.addEventListener("click", (event) => {
      if (event.target === this.overlayEl) {
        this.requestClose();
      }
    });
    this.footerSlotEl.addEventListener("slotchange", () => this.renderFooter());
  }

  connectedCallback(): void {
    document.addEventListener("keydown", this.handleKeydown);
    this.render();
  }

  disconnectedCallback(): void {
    document.removeEventListener("keydown", this.handleKeydown);
  }

  attributeChangedCallback(): void {
    this.render();
  }

  get open(): boolean {
    return this.hasAttribute("open");
  }

  set open(value: boolean) {
    this.toggleAttribute("open", value);
  }

  get title(): string {
    return this.getAttribute("title") ?? "";
  }

  set title(value: string) {
    this.setAttribute("title", value);
  }

  private handleKeydown = (event: KeyboardEvent): void => {
    if (event.key === "Escape" && this.open) {
      this.requestClose();
    }
  };

  private requestClose(): void {
    this.dispatchEvent(
      new CustomEvent("close", { bubbles: true, composed: true }),
    );
  }

  private render(): void {
    this.titleEl.textContent = this.title;
    this.dialogEl.setAttribute("aria-label", this.title);
    this.renderFooter();
  }

  private renderFooter(): void {
    const assigned = this.footerSlotEl.assignedNodes({ flatten: true });
    const hasContent = assigned.some(
      (node) => node.nodeType !== Node.TEXT_NODE || (node.textContent ?? "").trim() !== "",
    );
    this.footerEl.toggleAttribute("hidden", !hasContent);
  }
}
