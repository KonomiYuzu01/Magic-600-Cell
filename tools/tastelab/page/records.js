export const PAGE = 500;
export const TIE_LIMIT = 1000;

// Every stored comparison in answer order, read in pages by time stamp.
export async function loadRecords(db) {
  const records = [], ids = [], seen = new Set();
  const collection = db.collection("comparisons");
  const add = (docs) => {
    for (const doc of docs) {
      if (seen.has(doc.id)) continue;
      seen.add(doc.id);
      records.push(doc.data());
      ids.push(doc.id);
    }
  };
  let last = null;
  for (;;) {
    let query = collection.orderBy("t").limit(PAGE);
    if (last !== null) query = query.where("t", ">", last);
    const snap = await query.get();
    if (snap.size < PAGE) {
      add(snap.docs);
      return { records, ids };
    }
    last = snap.docs.at(-1).data().t;
    // Finish the boundary group in id order before advancing past its time.
    add(snap.docs.filter((doc) => doc.data().t < last));
    const ties = await collection.where("t", "==", last).limit(TIE_LIMIT).get();
    if (ties.size === TIE_LIMIT) throw Object.assign(new Error("The timestamp tie group may be incomplete."), { code: "tie_overflow" });
    add(ties.docs);
  }
}
