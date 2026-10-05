import { nextImage, isReady, fitLogistic, suggestPairs } from "./images.js";

export function handleMessage(message) {
  const { id, order, vectors, ratings, skips, ratedCount, notedPairs } = message;
  try {
    // Ratings of closed bundles stay in storage, but cannot train without vectors.
    const available = new Map(order.filter((imageId) => ratings.has(imageId)).map((imageId) => [imageId, ratings.get(imageId)]));
    const ready = isReady(available);
    let model = null;
    if (ready) {
      const ids = [...available.keys()];
      model = fitLogistic(ids.map((imageId) => vectors.get(imageId)), ids.map((imageId) => available.get(imageId) === "like" ? 1 : 0));
    }
    return {
      type: "suggestions", id, ready,
      next: nextImage({ order, vectors, ratings: available, skips, ratedCount, model }),
      pairs: suggestPairs({ vectors, ratings: available, notedPairs, limit: 10 }),
    };
  } catch (error) {
    return { type: "error", id, message: error.message };
  }
}

if (typeof WorkerGlobalScope !== "undefined" && self instanceof WorkerGlobalScope) {
  self.onmessage = (event) => self.postMessage(handleMessage(event.data));
}
