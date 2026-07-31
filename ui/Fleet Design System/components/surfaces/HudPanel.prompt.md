Translucent graphite panel with a hairline outline — use it for every piece of HUD chrome that sits over the battlefield; never for content inside the playfield.

```jsx
<HudPanel title="Front · Kestrel Gate" meta="2 fleets" tone="friendly" brackets>
  <FleetChip id="04" strength={0.62} state="committed" />
</HudPanel>
```

Variants: `tone` recolours only the outline (neutral / friendly / enemy). `brackets` marks the panel as the current selection. `dense` for sub-panels nested inside another panel.
