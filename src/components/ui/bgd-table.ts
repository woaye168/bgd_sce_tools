/**
 * <bgd-table> 表格
 * 属性：columns — JSON 字符串，格式 [{ key: string, title: string, sortable?: boolean }]；
 *       rows — JSON 字符串，格式 Array<Record<string, unknown>>；
 *       sort-key / sort-order（asc | desc）— 当前排序状态（只读展示，点击表头触发 sort 事件）
 * 事件：sort（detail: { key: string, order: "asc" | "desc" }，bubbles + composed）
 * 说明：组件只负责展示与抛出排序意图，实际排序由使用者处理 rows 后重新传入。
 */
interface TableColumn {
  key: string;
  title: string;
  sortable?: boolean;
}

type SortOrder = "asc" | "desc";

const template = document.createElement("template");
template.innerHTML = `
  <style>
    :host {
      display: block;
      overflow-x: auto;
    }
    table {
      width: 100%;
      border-collapse: collapse;
      font-size: 0.875rem;
      font-family: inherit;
      color: #e2e8f0; /* slate-200 */
    }
    th, td {
      text-align: left;
      padding: 0.5rem 0.75rem;
      border-bottom: 1px solid #334155; /* slate-700 */
      white-space: nowrap;
    }
    th {
      color: #94a3b8; /* slate-400 */
      font-weight: 600;
      user-select: none;
    }
    th.sortable {
      cursor: pointer;
    }
    th.sortable:hover {
      color: #e2e8f0;
    }
    th .arrow {
      margin-left: 0.25rem;
      font-size: 0.75rem;
    }
    tbody tr:hover {
      background: #33415566; /* slate-700 / 40% */
    }
    .empty {
      padding: 1.5rem 0.75rem;
      text-align: center;
      color: #64748b; /* slate-500 */
    }
  </style>
  <table part="table">
    <thead><tr class="head-row"></tr></thead>
    <tbody class="body"></tbody>
  </table>
`;

export class BgdTable extends HTMLElement {
  static get observedAttributes(): string[] {
    return ["columns", "rows", "sort-key", "sort-order"];
  }

  private headRowEl: HTMLTableRowElement;
  private bodyEl: HTMLTableSectionElement;

  constructor() {
    super();
    const root = this.attachShadow({ mode: "open" });
    root.appendChild(template.content.cloneNode(true));
    this.headRowEl = root.querySelector(".head-row") as HTMLTableRowElement;
    this.bodyEl = root.querySelector(".body") as HTMLTableSectionElement;
  }

  connectedCallback(): void {
    this.render();
  }

  attributeChangedCallback(): void {
    this.render();
  }

  get columns(): TableColumn[] {
    return this.parseJsonArray<TableColumn>(this.getAttribute("columns"));
  }

  set columns(value: TableColumn[]) {
    this.setAttribute("columns", JSON.stringify(value));
  }

  get rows(): Array<Record<string, unknown>> {
    return this.parseJsonArray<Record<string, unknown>>(this.getAttribute("rows"));
  }

  set rows(value: Array<Record<string, unknown>>) {
    this.setAttribute("rows", JSON.stringify(value));
  }

  get sortKey(): string {
    return this.getAttribute("sort-key") ?? "";
  }

  get sortOrder(): SortOrder {
    return this.getAttribute("sort-order") === "desc" ? "desc" : "asc";
  }

  private parseJsonArray<T>(raw: string | null): T[] {
    if (!raw) {
      return [];
    }
    try {
      const parsed: unknown = JSON.parse(raw);
      return Array.isArray(parsed) ? (parsed as T[]) : [];
    } catch {
      return [];
    }
  }

  private render(): void {
    const columns = this.columns;
    const rows = this.rows;

    this.headRowEl.replaceChildren(
      ...columns.map((column) => {
        const th = document.createElement("th");
        th.textContent = column.title;
        if (column.sortable) {
          th.classList.add("sortable");
          if (column.key === this.sortKey) {
            const arrow = document.createElement("span");
            arrow.className = "arrow";
            arrow.textContent = this.sortOrder === "asc" ? "▲" : "▼";
            th.appendChild(arrow);
          }
          th.addEventListener("click", () => {
            const order: SortOrder =
              column.key === this.sortKey && this.sortOrder === "asc" ? "desc" : "asc";
            this.dispatchEvent(
              new CustomEvent("sort", {
                detail: { key: column.key, order },
                bubbles: true,
                composed: true,
              }),
            );
          });
        }
        return th;
      }),
    );

    if (rows.length === 0) {
      const tr = document.createElement("tr");
      const td = document.createElement("td");
      td.className = "empty";
      td.colSpan = Math.max(columns.length, 1);
      td.textContent = "暂无数据";
      tr.appendChild(td);
      this.bodyEl.replaceChildren(tr);
      return;
    }

    this.bodyEl.replaceChildren(
      ...rows.map((row) => {
        const tr = document.createElement("tr");
        for (const column of columns) {
          const td = document.createElement("td");
          const value = row[column.key];
          td.textContent = value === null || value === undefined ? "" : String(value);
          tr.appendChild(td);
        }
        return tr;
      }),
    );
  }
}
