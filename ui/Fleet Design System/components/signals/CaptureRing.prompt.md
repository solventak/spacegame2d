The match's decisive objective readout. Place it on the Shield Relay in the playfield, not in a panel.

```jsx
<CaptureRing value={0.62} segments={8} tone="friendly" label="5.0" sub="relay" />
<CaptureRing value={0.5} mode="contested" tone="enemy" label="4.0" sub="contested" />
```

`mode="decay"` renders the neutral gray reverse-drain used when the capturing team leaves.
