const state = {
    table: null,
    primaryKey: null,
    offset: 0,
    limit: 25,
    searchTimer: null,
    requestId: 0,
    auditOffset: 0,
};

const AUDIT_PAGE_SIZE = 50;

async function api(path) {
    const r = await fetch(path, {
        credentials: "same-origin"
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
    state.primaryKey = null;
    state.offset = 0;

    document
        .querySelectorAll(".classes button")
        .forEach((b) => b.classList.remove("active"));

    btn.classList.add("active");

    document.getElementById("q").disabled = false;

    const schema = await api(
        `/school/api/tables/${encodeURIComponent(table)}/schema`
    );

    const primaryKeys = schema.columns.filter((column) => column.pk);

    if (primaryKeys.length === 1) {
        state.primaryKey = primaryKeys[0].name;
    }

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
                const studentId =
                    state.primaryKey != null ? row[state.primaryKey] : null;

                const rowClass = studentId != null ? "student-row" : "";

                const dataAttribute =
                    studentId != null ?
                    ` data-student-id="${esc(studentId)}"` :
                    "";

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

        document.getElementById("out").innerHTML =
            `<p class="err">${esc(error.message)}</p>`;
    }
}

async function loadStudent(studentId, view = "auto") {
    if (!state.table) return;

    const table = encodeURIComponent(state.table);
    const student = encodeURIComponent(studentId);

    try {
        let data;
        let isAdmin = false;

        if (view === "admin") {
            // Deliberate request for the full record -- no fallback. A
            // genuinely non-admin user gets a real 403 here, which is
            // correct: they explicitly asked for the admin view.
            data = await api(
                `/school/api/admin/tables/${table}/students/${student}`
            );
            isAdmin = true;
        } else if (view === "privacy") {
            // Deliberate request for the privacy-filtered view. This is
            // the toggle schooladmin needs -- without it, the admin
            // endpoint always succeeds for them and the filtered branch
            // below is never reached.
            data = await api(
                `/school/api/tables/${table}/students/${student}`
            );
        } else {
            // Initial load (row click): try admin first, fall back to
            // the privacy-filtered endpoint on 403. Unchanged from
            // before -- this is what makes the view "just work" before
            // we know whether the current user is an admin.
            try {
                data = await api(
                    `/school/api/admin/tables/${table}/students/${student}`
                );
                isAdmin = true;
            } catch (error) {
                if (error.status !== 403) {
                    throw error;
                }

                data = await api(
                    `/school/api/tables/${table}/students/${student}`
                );
            }
        }

        renderStudent(data, isAdmin, studentId);
    } catch (error) {
        document.getElementById("out").innerHTML =
            `<p class="err">${esc(error.message)}</p>`;
    }
}

async function loadAuditLog(offset = 0) {
    state.auditOffset = offset;

    try {
        const data = await api(
            `/school/api/admin/audit?limit=${AUDIT_PAGE_SIZE}&offset=${offset}`
        );

        renderAuditLog(data);
    } catch (error) {
        document.getElementById("out").innerHTML =
            `<p class="err">${esc(error.message)}</p>`;
    }

    document.getElementById("meta").textContent = "";
}

function renderAuditLog(data) {
    const events = data.events;

    let html = `
    <div class="audit-header">
      <button type="button" id="back-from-audit">← Back</button>
      <h2>Admin Audit Log</h2>
    </div>
  `;

    if (events.length === 0) {
        html += `<p class="err">No audit events${state.auditOffset > 0 ? " on this page" : " yet"}.</p>`;
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
      <button type="button" id="audit-prev" ${data.offset <= 0 ? "disabled" : ""}>Prev</button>
      <button type="button" id="audit-next" ${events.length < data.limit ? "disabled" : ""}>Next</button>
    </div>
  `;

    document.getElementById("out").innerHTML = html;

    document.getElementById("back-from-audit").onclick = () => {
        if (state.table) {
            loadPage();
        } else {
            document.getElementById("out").innerHTML = "";
        }
    };

    document.getElementById("audit-prev").onclick = () => {
        loadAuditLog(Math.max(0, data.offset - data.limit));
    };

    document.getElementById("audit-next").onclick = () => {
        loadAuditLog(data.offset + data.limit);
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
        <button type="button" id="view-admin" class="${isAdmin ? "active" : ""}">Full Record</button>
        <button type="button" id="view-privacy" class="${isAdmin ? "" : "active"}">Privacy View</button>
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
    state.offset = Math.max(0, state.offset - state.limit);
    loadPage();
};

document.getElementById("next").onclick = () => {
    state.offset += state.limit;
    loadPage();
};

document.getElementById("audit-log-link").onclick = () => {
    loadAuditLog(0);
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