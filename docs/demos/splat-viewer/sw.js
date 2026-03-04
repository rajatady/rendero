// Service Worker — caches .splat files for instant reload
const CACHE_NAME = 'splat-cache-v1';

self.addEventListener('fetch', (event) => {
    const url = event.request.url;
    // Only cache .splat file requests
    if (!url.endsWith('.splat')) return;

    event.respondWith(
        caches.open(CACHE_NAME).then(async (cache) => {
            const cached = await cache.match(event.request);
            if (cached) return cached;
            const response = await fetch(event.request);
            if (response.ok) cache.put(event.request, response.clone());
            return response;
        })
    );
});
