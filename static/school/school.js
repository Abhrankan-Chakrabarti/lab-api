const state = {
  table: null,
  offset: 0,
  limit: 25,
  searchTimer: null,
  requestId: 0,
};

async function api(path) {
  const r = await fetch(path, { credentials: "same-origin" });

  if (!r.ok) {
    throw new Error(`${r.status} ${await r.text()}`);
  }

  return r.json();
}

function esc(s) {
  return String(s ?? "").replace(/[&<>"']/g, (c) =>
    ({
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      '"': "&quot;",
      "'": "&#39;",
    })[c]
  );
}

async function loadTables() {
  const data = await api("/school/api/tables");
  const el = document.getElementById("classes");

  el.innerHTML = "";

  data.tables.forEach((t) => {
    const b = document.createElement("button");

    b.textContent = t.replaceAll("_", "-");
    b.onclick = () => selectTable(t, b);

    el.appendChild(b);
  });
}

async function selectTable(table, btn) {
  state.table = table;
  state.offset = 0;

  document
    .querySelectorAll(".classes button")
    .forEach((b) => b.classList.remove("active"));

  btn.classList.add("active");

  document.getElementById("q").disabled = false;

  await loadPage();
}

async function loadPage() {
  if (!state.table) return;

  const requestId = ++state.requestId;
  const q = document.getElementById("q").value.trim();

  const params = new URLSearchParams({
    limit: String(state.limit),
    offset: String(state.offset),
  });

  if (q) {
    params.set("search", q);
  }

  try {
    const data = await api(
      `/school/api/tables/${encodeURIComponent(state.table)}?${params}`
    );

    // Ignore an older response if a newer request has already started.
    if (requestId !== state.requestId) return;

    document.getElementById("meta").textContent =
      `${data.table} · showing ${data.returned} of ${data.total} ` +
      `(offset ${data.offset})`;

    const prefer = [
      "Roll No",
      "Student Code",
      "Student Name",
      "Student DOB",
    ];

    let cols = prefer.filter((c) => data.columns.includes(c));

    if (cols.length === 0) {
      cols = data.columns.slice(0, 5);
    }

    if (data.rows.length === 0) {
      document.getElementById("out").innerHTML =
        "<p class='err'>No rows</p>";
    } else {
      let html =
        "<table><thead><tr>" +
        cols.map((c) => `<th>${esc(c)}</th>`).join("") +
        "</tr></thead><tbody>";

      for (const row of data.rows) {
        const studentCode = row["Student Code"];

        const rowClass = studentCode != null ? "student-row" : "";

        const dataAttribute =
          studentCode != null
            ? ` data-student-code="${esc(studentCode)}"`
            : "";

        html += `<tr class="${rowClass}"${dataAttribute}>`;

        html += cols
          .map((c) => `<td>${esc(row[c])}</td>`)
          .join("");

        html += "</tr>";
      }

      html += "</tbody></table>";

      document.getElementById("out").innerHTML = html;

      document.querySelectorAll(".student-row").forEach((row) => {
        row.addEventListener("click", () => {
          const studentCode = row.dataset.studentCode;

          if (studentCode) {
            loadStudent(studentCode);
          }
        });
      });
    }

    updatePagination(data);
  } catch (error) {
    if (requestId !== state.requestId) return;

    document.getElementById("out").innerHTML =
      `<p class="err">${esc(error.message)}</p>`;
  }
}

async function loadStudent(studentCode) {
  if (!state.table) return;

  try {
    const data = await api(
      `/school/api/tables/${encodeURIComponent(
        state.table
      )}/students/${encodeURIComponent(studentCode)}`
    );

    renderStudent(data);
  } catch (error) {
    document.getElementById("out").innerHTML =
      `<p class="err">${esc(error.message)}</p>`;
  }
}

function renderStudent(data) {
  const student = data.student;

  let html = `
    <div class="student-detail">
      <div class="student-detail-header">
        <button type="button" id="back-to-table">← Back</button>
        <h2>Student Details</h2>
      </div>
      <dl>
  `;

  for (const [key, value] of Object.entries(student)) {
    html += `
      <dt>${esc(key)}</dt>
      <dd>${esc(value)}</dd>
    `;
  }

  html += `
      </dl>
    </div>
  `;

  document.getElementById("out").innerHTML = html;

  document.getElementById("meta").textContent =
    `${data.table} · Student ${student["Student Code"] ?? ""}`;

  document.getElementById("back-to-table").onclick = () => {
    loadPage();
  };
}

function updatePagination(data) {
  const prev = document.getElementById("prev");
  const next = document.getElementById("next");

  prev.disabled = data.offset <= 0;
  next.disabled = data.offset + data.returned >= data.total;
}

document.getElementById("prev").onclick = () => {
  state.offset = Math.max(0, state.offset - state.limit);
  loadPage();
};

document.getElementById("next").onclick = () => {
  state.offset += state.limit;
  loadPage();
};

document.getElementById("q").addEventListener("input", () => {
  clearTimeout(state.searchTimer);

  state.searchTimer = setTimeout(() => {
    state.offset = 0;
    loadPage();
  }, 300);
});

loadTables().catch((e) => {
  document.getElementById("out").innerHTML =
    `<p class="err">${esc(e.message)}</p>`;
});