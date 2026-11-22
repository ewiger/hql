// The playground (HQL-0061). The module that runs HQL is fetched the first
// time a reader presses Run or Check; a page that is never run downloads
// nothing. It sits beside this script, so the address holds under every
// published version of the site.

let loading = null;

function hql() {
  if (!loading) {
    const url = new URL("wasm/hql_wasm.js", import.meta.url);
    loading = import(url.href).then(async (module) => {
      await module.default();
      return module;
    });
    // A failed load is tried again on the next press.
    loading.catch(() => {
      loading = null;
    });
  }
  return loading;
}

// What `hql` would print: diagnostics, then the value.
async function run(source, how) {
  try {
    const module = await hql();
    const report = how === "check" ? module.check(source) : module.run(source);
    const text = (report.diagnostics + report.output).replace(/\s+$/, "");
    const ok = report.ok;
    report.free();
    return { ok, text: text || "(no output)" };
  } catch (error) {
    // A trap leaves the module in an unknown state: load a fresh one.
    loading = null;
    return { ok: false, text: `the playground stopped: ${error.message || error}` };
  }
}

function button(label, how) {
  const made = document.createElement("button");
  made.type = "button";
  made.className = "hql-run";
  made.textContent = label;
  made.dataset.how = how;
  return made;
}

async function press(pressed, source, shown) {
  const label = pressed.textContent;
  pressed.disabled = true;
  pressed.textContent = "Running…";
  const result = await run(source, pressed.dataset.how);
  shown.textContent = result.text;
  shown.classList.toggle("hql-failed", !result.ok);
  shown.classList.add("hql-live");
  pressed.textContent = label;
  pressed.disabled = false;
}

// A program that names a configured endpoint needs its server; the page
// shows its recorded output and offers no button.
function reaches_a_server(source) {
  return /\b(tap|sink|source|target)\(\s*"/.test(source);
}

// The front page: each program of the demo window, with the output block
// that follows it.
function demo() {
  for (const tab of document.querySelectorAll(".hql-term .tabbed-block")) {
    // A tab holds a program and, after it, what the program printed.
    const [program, output] = tab.querySelectorAll(".highlight");
    const code = program && program.querySelector("code");
    if (!code || !output) continue;
    const source = code.textContent;
    if (reaches_a_server(source)) continue;
    const shown = output.querySelector("code");
    const made = button("Run", "eval");
    made.addEventListener("click", () => press(made, source, shown));
    program.after(made);
  }
}

// The playground page: an editor, two buttons, an output pane and a menu of
// programs. The programs are the titled code blocks of `.hql-examples`,
// which a reader without JavaScript sees as they are.
function playground() {
  for (const root of document.querySelectorAll("[data-hql-playground]")) {
    const editor = root.querySelector("textarea");
    const shown = root.querySelector("pre.hql-output");
    const menu = root.querySelector("select");
    const examples = [];
    for (const listed of document.querySelectorAll(".hql-examples")) {
      for (const block of listed.querySelectorAll(".highlight")) {
        const title = block.querySelector(".filename");
        const code = block.querySelector("pre code");
        if (title && code) examples.push({ title: title.textContent, source: code.textContent });
      }
      listed.hidden = true;
    }
    for (const example of examples) {
      const option = document.createElement("option");
      option.textContent = example.title;
      menu.append(option);
    }
    const load = () => {
      const example = examples[menu.selectedIndex];
      if (example) editor.value = example.source.replace(/\s+$/, "\n");
    };
    menu.addEventListener("change", load);
    load();
    for (const pressed of root.querySelectorAll("button[data-how]")) {
      pressed.addEventListener("click", () => press(pressed, editor.value, shown));
    }
    // Ctrl or Cmd with Enter runs; Tab indents rather than leaving the editor.
    editor.addEventListener("keydown", (event) => {
      if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
        event.preventDefault();
        root.querySelector('button[data-how="eval"]').click();
      } else if (event.key === "Tab" && !event.shiftKey) {
        event.preventDefault();
        editor.setRangeText("    ", editor.selectionStart, editor.selectionEnd, "end");
      }
    });
    root.hidden = false;
  }
}

demo();
playground();
