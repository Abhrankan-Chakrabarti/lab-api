const state = { table: null, offset: 0, limit: 25 };

async function api(path) {
  const r = await fetch(path, { credentials: "same-origin" });
  if (!r.ok) throw new Error(`${r.status} ${await r.text()}`);
  return r.json();
}

function esc(s) {
  return String(s ?? "").replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c])
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
  document.querySelectorAll(".classes button").forEach((b) => b.classList.remove("active"));
  btn.classList.add("active");
  document.getElementById("q").disabled = false;
  await loadPage();
}

async function loadPage() {
  if (!state.table) return;
  const q = document.getElementById("q").value.trim();
  const params = new URLSearchParams({
    limit: String(state.limit),
    offset: String(state.offset),
  });
  if (q) params.set("search", q);
  const data = await api(`/school/api/tables/${encodeURIComponent(state.table)}?${params}`);
  document.getElementById("meta").textContent =
    `${data.table} · showing ${data.returned} (offset ${data.offset})`;

  // Prefer a few useful columns if present; else first 5
  const prefer = ["Roll No", "Student Code", "Student Name", "Student DOB"];
  let cols = prefer.filter((c) => data.columns.includes(c));
  if (cols.length === 0) cols = data.columns.slice(0, 5);

  let html = "<table><thead><tr>" + cols.map((c) => `<th>${esc(c)}</th>`).join("") + "</tr></thead><tbody>";
  for (const row of data.rows) {
    html += "<tr>" + cols.map((c) => `<td>${esc(row[c])}</td>`).join("") + "</tr>";
  }
  html += "</tbody></table>";
  document.getElementById("out").innerHTML = html || "<p class='err'>No rows</p>";
  document.getElementById("prev").disabled = state.offset <= 0;
  document.getElementById("next").disabled = data.returned < state.limit;
}

document.getElementById("prev").onclick = () => {
  state.offset = Math.max(0, state.offset - state.limit);
  loadPage();
};
document.getElementById("next").onclick = () => {
  state.offset += state.limit;
  loadPage();
};
document.getElementById("q").addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    state.offset = 0;
    loadPage();
  }
});

loadTables().catch((e) => {
  document.getElementById("out").innerHTML = `<p class="err">${esc(e.message)}</p>`;
});