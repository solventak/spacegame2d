The only permanently-visible chrome across the top. Keep it 34px tall and keep the centre reserved for the decisive objective.

```jsx
<ObjectiveBar
  left={<Readout label="drones" value="4,280" />}
  right={<ConfidenceTag level="estimated">enemy seen 32s ago</ConfidenceTag>}
  objective={{ value: .62, elapsed: 5.0, mode: 'contested', tone: 'enemy',
    cores: [{ label: 'YOU', tone: 'friendly', shielded: true }, { label: 'OPP', tone: 'enemy', shielded: false }] }} />
```
