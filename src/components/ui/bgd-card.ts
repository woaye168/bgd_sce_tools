/**
 * <bgd-card> 卡片容器
 * 属性：title — 卡片标题
 * 插槽：默认 slot — 卡片内容
 */
const template = document.createElement("template");
template.innerHTML = `
  <style>
    :host {
      display: block;
    }
    .card {
      background: #1e293b; /* slate-800 */
      border: 1px solid #334155; /* slate-700 */
      border-radius: 0.75rem; /* rounded-xl */
      padding: 1.25rem;
      color: #e2e8f0; /* slate-200 */
      font-family: inherit;
    }
    .card__title {
      margin: 0 0 1rem;
      font-size: 1rem;
      font-weight: 600;
      color: #f1f5f9; /* slate-100 */
    }
    .card__title[hidden] {
      display: none;
    }
  </style>
  <section class="card">
    <h2 class="card__title" part="title"></h2>
    <div class="card__body" part="body">
      <slot></slot>
    </div>
  </section>
`;

export class BgdCard extends HTMLElement {
  static get observedAttributes(): string[] {
    return ["title"];
  }

  private titleEl: HTMLElement;

  constructor() {
    super();
    const root = this.attachShadow({ mode: "open" });
    root.appendChild(template.content.cloneNode(true));
    this.titleEl = root.querySelector(".card__title") as HTMLElement;
  }

  connectedCallback(): void {
    this.render();
  }

  attributeChangedCallback(): void {
    this.render();
  }

  get title(): string {
    return this.getAttribute("title") ?? "";
  }

  set title(value: string) {
    this.setAttribute("title", value);
  }

  private render(): void {
    const title = this.title;
    this.titleEl.textContent = title;
    this.titleEl.toggleAttribute("hidden", title === "");
  }
}
