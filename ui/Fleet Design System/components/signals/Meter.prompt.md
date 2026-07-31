Fleet's linear bar: fleet strength, capture progress, shield integrity. The `mode` is load-bearing — stripes always mean paused/contested, gray stripes always mean decaying.

```jsx
<Meter value={0.62} tone="friendly" />
<Meter value={0.48} tone="enemy" mode="contested" />
<Meter value={0.31} mode="decay" tone="neutral" />
```
