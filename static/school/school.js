const state = {
    table: null,
    primaryKey: null,
    offset: 0,
    limit: 25,
    searchTimer: null,
    requestId: 0,
    auditOffset: 0,
    view: "table", // "table" | "student" | "audit"
};

const AUDIT_PAGE_SIZE = 50;

async function api(path) {
    const r = await fetch(path, {
        credentials: "same-origin",
    });

    if (!r.ok) {
        const error = new Error(`${r.status} ${await r.text()}`);
        error.status = r.status;
        throw error;
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
        })[c],
    );
}

function setTableControlsEnabled(enabled) {
    const search = document.getElementById("q");
    const prev = document.getElementById("prev");
    const next = document.getElementById("next");

    // Search stays disabled until a class table is selected.
    search.disabled = !enabled || !state.table;
    prev.disabled = !enabled;
    next.disabled = !enabled;
}

async function probeAdminAuditAccess() {
    const link = document.getElementById("audit-log-link");
    try {
        const r = await fetch("/school/api/admin/audit?limit=1&offset=0", {
            credentials: "same-origin",
        });
        if (r.status === 403 || r.status === 401) {
            link.hidden = true;
            return;
        }
        // 200, 503, etc. — show the button; loadAuditLog handles errors
        link.hidden = false;
    } catch {
        link.hidden = true;
    }
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
    state.primaryKey = null;
    state.offset = 0;
    state.view = "table";

    document
        .querySelectorAll(".classes button")
        .forEach((b) => b.classList.remove("active"));
    btn.classList.add("active");

    const schema = await api(
        `/school/api/tables/${encodeURIComponent(table)}/schema`,
    );

    const primaryKeys = schema.columns.filter((column) => column.pk);
    if (primaryKeys.length === 1) {
        state.primaryKey = primaryKeys[0].name;
    }

    await loadPage();
}

async function loadPage() {
    if (!state.table) return;

    state.view = "table";
    setTableControlsEnabled(true);

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
            `/school/api/tables/${encodeURIComponent(state.table)}?${params}`,
        );

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
                const studentId =
                    state.primaryKey != null ? row[state.primaryKey] : null;
                const rowClass = studentId != null ? "student-row" : "";
                const dataAttribute =
                    studentId != null
                        ? ` data-student-id="${esc(studentId)}"`
                        : "";

                html += `<tr class="${rowClass}"${dataAttribute}>`;
                html += cols.map((c) => `<td>${esc(row[c])}</td>`).join("");
                html += "</tr>";
            }

            html += "</tbody></table>";
            document.getElementById("out").innerHTML = html;

            document.querySelectorAll(".student-row").forEach((row) => {
                row.addEventListener("click", () => {
                    const studentId = row.dataset.studentId;
                    if (studentId) {
                        loadStudent(studentId);
                    }
                });
            });
        }

        updatePagination(data);
    } catch (error) {
        if (requestId !== state.requestId) return;
        setTableControlsEnabled(false);
        document.getElementById("out").innerHTML =
            `<p class="err">${esc(error.message)}</p>`;
    }
}

async function loadStudent(studentId, view = "auto") {
    if (!state.table) return;

    state.view = "student";
    setTableControlsEnabled(false);

    const table = encodeURIComponent(state.table);
    const student = encodeURIComponent(studentId);

    try {
        let data;
        let isAdmin = false;

        if (view === "admin") {
            data = await api(
                `/school/api/admin/tables/${table}/students/${student}`,
            );
            isAdmin = true;
        } else if (view === "privacy") {
            data = await api(
                `/school/api/tables/${table}/students/${student}`,
            );
        } else {
            try {
                data = await api(
                    `/school/api/admin/tables/${table}/students/${student}`,
                );
                isAdmin = true;
            } catch (error) {
                if (error.status !== 403) {
                    throw error;
                }
                data = await api(
                    `/school/api/tables/${table}/students/${student}`,
                );
            }
        }

        renderStudent(data, isAdmin, studentId);
    } catch (error) {
        let msg = error.message;
        if (error.status === 403) {
            msg = "Admin access required for the full record.";
        } else if (error.status === 503) {
            msg =
                "Admin detail is unavailable (audit logging not configured).";
        }
        document.getElementById("out").innerHTML =
            `<p class="err">${esc(msg)}</p>`;
    }
}

async function loadAuditLog(offset = 0) {
    state.auditOffset = offset;
    state.view = "audit";
    setTableControlsEnabled(false);

    try {
        const data = await api(
            `/school/api/admin/audit?limit=${AUDIT_PAGE_SIZE}&offset=${offset}`,
        );
        renderAuditLog(data);
    } catch (error) {
        let msg = error.message;
        if (error.status === 403) {
            msg = "Admin access required to view the audit log.";
        } else if (error.status === 401) {
            msg = "Authentication required.";
        } else if (error.status === 503) {
            msg = "Audit logging is unavailable on the server.";
        }
        document.getElementById("meta").textContent = "";
        document.getElementById("out").innerHTML =
            `<p class="err">${esc(msg)}</p>`;
    }
}

function renderAuditLog(data) {
    const events = data.events || [];
    const limit = data.limit ?? AUDIT_PAGE_SIZE;
    const offset = data.offset ?? 0;

    document.getElementById("meta").textContent =
        `Audit log · ${events.length} event(s) on this page · offset ${offset} · limit ${limit}`;

    let html = `
    <div class="audit-header">
      <button type="button" id="back-from-audit">← Back</button>
      <h2>Admin Audit Log</h2>
    </div>
  `;

    if (events.length === 0) {
        html += `<p class="err">No audit events${
            offset > 0 ? " on this page" : " yet"
        }.</p>`;
    } else {
        html += `
      <table>
        <thead>
          <tr>
            <th>Timestamp (UTC)</th>
            <th>User</th>
            <th>Table</th>
            <th>Student Code</th>
            <th>Action</th>
          </tr>
        </thead>
        <tbody>
    `;

        for (const event of events) {
            html += `
        <tr>
          <td>${esc(event.timestamp)}</td>
          <td>${esc(event.user)}</td>
          <td>${esc(event.table)}</td>
          <td>${esc(event.student_code)}</td>
          <td>${esc(event.action)}</td>
        </tr>
      `;
        }

        html += `</tbody></table>`;
    }

    html += `
    <div class="audit-pager">
      <button type="button" id="audit-prev" ${
          offset <= 0 ? "disabled" : ""
      }>Prev</button>
      <button type="button" id="audit-next" ${
          events.length < limit ? "disabled" : ""
      }>Next</button>
    </div>
  `;

    document.getElementById("out").innerHTML = html;

    document.getElementById("back-from-audit").onclick = () => {
        if (state.table) {
            loadPage();
        } else {
            state.view = "table";
            document.getElementById("meta").textContent = "";
            document.getElementById("out").innerHTML = "";
            setTableControlsEnabled(false);
        }
    };

    document.getElementById("audit-prev").onclick = () => {
        loadAuditLog(Math.max(0, offset - limit));
    };

    document.getElementById("audit-next").onclick = () => {
        loadAuditLog(offset + limit);
    };
}

function renderStudent(data, isAdmin = false, studentId = null) {
    const student = data.student;
    const accessBadge = isAdmin
        ? '<span class="admin-badge">Admin · full record</span>'
        : '<span class="privacy-badge">Privacy-filtered</span>';

    let html = `
    <div class="student-detail">
      <div class="student-detail-header">
        <button type="button" id="back-to-table">← Back</button>
        <div>
          <h2>Student Details</h2>
          ${accessBadge}
        </div>
      </div>
      <div class="view-toggle">
        <button type="button" id="view-admin" class="${
            isAdmin ? "active" : ""
        }">Full Record</button>
        <button type="button" id="view-privacy" class="${
            isAdmin ? "" : "active"
        }">Privacy View</button>
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

    const displayId =
        state.primaryKey != null ? student[state.primaryKey] : "";

    document.getElementById("meta").textContent =
        `${data.table} · Student ${displayId ?? ""}`;

    document.getElementById("back-to-table").onclick = () => {
        loadPage();
    };

    document.getElementById("view-admin").onclick = () => {
        loadStudent(studentId, "admin");
    };

    document.getElementById("view-privacy").onclick = () => {
        loadStudent(studentId, "privacy");
    };
}

function updatePagination(data) {
    const prev = document.getElementById("prev");
    const next = document.getElementById("next");
    prev.disabled = data.offset <= 0;
    next.disabled = data.offset + data.returned >= data.total;
}

document.getElementById("prev").onclick = () => {
    if (state.view !== "table") return;
    state.offset = Math.max(0, state.offset - state.limit);
    loadPage();
};

document.getElementById("next").onclick = () => {
    if (state.view !== "table") return;
    state.offset += state.limit;
    loadPage();
};

document.getElementById("audit-log-link").onclick = () => {
    loadAuditLog(0);
};

document.getElementById("q").addEventListener("input", () => {
    clearTimeout(state.searchTimer);
    state.searchTimer = setTimeout(() => {
        if (state.view !== "table") return;
        state.offset = 0;
        loadPage();
    }, 300);
});

Promise.all([loadTables(), probeAdminAuditAccess()]).catch((e) => {
    document.getElementById("out").innerHTML =
        `<p class="err">${esc(e.message)}</p>`;
});