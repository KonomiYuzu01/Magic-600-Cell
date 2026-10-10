export async function chooseStorage({ claude, openLocal }) {
  if (claude) return { mode: "artifact", standalone: false };
  try {
    const db = await openLocal();
    return { mode: "standalone", standalone: true, db };
  } catch {
    return { mode: "none", standalone: true };
  }
}

export async function saveFile({ downloads, standalone, document, URL, Blob, schedule = setTimeout }, filename, data) {
  if (downloads) {
    await downloads.save({ filename, data });
    return;
  }
  if (!standalone) throw Object.assign(new Error("Export is not available in this view."), { code: "unavailable" });
  const url = URL.createObjectURL(new Blob([data], { type: "application/json" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.hidden = true;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  schedule(() => URL.revokeObjectURL(url), 60000);
}
